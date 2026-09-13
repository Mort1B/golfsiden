mod commands;
pub mod completion;
pub mod reads;
pub mod setup;
use crate::domain::{
    match_play::{
        HandicapAllocation, MatchMode,
        commands::{AcceptedEvent, validate_ledger},
    },
    models::{RoundStatus, ScoringFormat, TournamentRole},
    scorecards::ScoreOwner,
};
use crate::repositories::{auth, score_authorization};
pub use commands::execute;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("resource not found")]
    NotFound,
    #[error("session is not authenticated")]
    Unauthenticated,
    #[error("match authority required")]
    Forbidden,
    #[error("{0}")]
    Invalid(&'static str),
    #[error("{0}")]
    Conflict(&'static str),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
#[derive(sqlx::FromRow)]
pub(super) struct Context {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub status: RoundStatus,
    pub scoring_format: ScoringFormat,
    pub handicap_enabled: bool,
    pub tee_id: Option<Uuid>,
    pub round_number: i16,
    pub number_of_rounds: i16,
    pub final_round_back_nine_hidden: bool,
}
#[derive(sqlx::FromRow)]
pub(super) struct Aggregate {
    pub id: Uuid,
    pub first_player_id: Uuid,
    pub second_player_id: Uuid,
    pub revision: i64,
    pub ledger: serde_json::Value,
    pub terminal: bool,
    pub confirmed: bool,
    pub correction_pending: bool,
}
impl Aggregate {
    fn players(&self) -> [Uuid; 2] {
        [self.first_player_id, self.second_player_id]
    }
    fn events(&self) -> Result<Vec<AcceptedEvent>, Error> {
        serde_json::from_value(self.ledger.clone())
            .map_err(|_| Error::Conflict("invalid stored ledger"))
    }
}
pub(super) async fn context(
    tx: &mut Transaction<'_, Postgres>,
    round: Uuid,
    lock: bool,
) -> Result<Context, Error> {
    let sql = if lock {
        "SELECT r.id,r.tournament_id,r.status,r.scoring_format,r.handicap_enabled,r.tee_id,r.round_number,t.number_of_rounds,t.final_round_back_nine_hidden FROM rounds r JOIN tournaments t ON t.id=r.tournament_id WHERE r.id=$1 FOR UPDATE OF r"
    } else {
        "SELECT r.id,r.tournament_id,r.status,r.scoring_format,r.handicap_enabled,r.tee_id,r.round_number,t.number_of_rounds,t.final_round_back_nine_hidden FROM rounds r JOIN tournaments t ON t.id=r.tournament_id WHERE r.id=$1"
    };
    let c = sqlx::query_as::<_, Context>(sql)
        .bind(round)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)?;
    if c.scoring_format != ScoringFormat::SinglesMatchPlay {
        return Err(Error::NotFound);
    }
    Ok(c)
}
pub(super) async fn aggregate(
    tx: &mut Transaction<'_, Postgres>,
    round: Uuid,
    id: Uuid,
    lock: bool,
) -> Result<Aggregate, Error> {
    let sql = if lock {
        "SELECT id,first_player_id,second_player_id,revision,ledger,terminal,confirmed,correction_pending FROM singles_matches WHERE id=$1 AND round_id=$2 FOR UPDATE"
    } else {
        "SELECT id,first_player_id,second_player_id,revision,ledger,terminal,confirmed,correction_pending FROM singles_matches WHERE id=$1 AND round_id=$2"
    };
    sqlx::query_as(sql)
        .bind(id)
        .bind(round)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)
}
pub(super) async fn live(tx: &mut Transaction<'_, Postgres>, session: Uuid) -> Result<Uuid, Error> {
    Ok(auth::lock_active_session(tx, session)
        .await?
        .ok_or(Error::Unauthenticated)?
        .user_id)
}
pub(super) async fn member(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    tournament: Uuid,
) -> Result<(Uuid, TournamentRole), Error> {
    let actor = live(tx, session).await?;
    let role = sqlx::query_scalar(
        "SELECT role FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR SHARE",
    )
    .bind(tournament)
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(Error::Forbidden)?;
    Ok((actor, role))
}
pub(super) async fn authority(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    c: &Context,
    m: &Aggregate,
) -> Result<(Uuid, bool), Error> {
    let (actor, role) = member(tx, session, c.tournament_id).await?;
    for player in m.players() {
        score_authorization::authorize_mutation(
            tx,
            session,
            c.id,
            c.scoring_format,
            ScoreOwner::Player { id: player },
        )
        .await
        .map_err(|e| match e {
            score_authorization::ScoreAuthorizationError::Unauthenticated => Error::Unauthenticated,
            score_authorization::ScoreAuthorizationError::Database(e) => Error::Database(e),
            _ => Error::Forbidden,
        })?;
    }
    Ok((actor, role == TournamentRole::Admin))
}
pub(super) async fn allocation(
    tx: &mut Transaction<'_, Postgres>,
    c: &Context,
    m: &Aggregate,
) -> Result<(HandicapAllocation, Vec<u8>), Error> {
    let mut handicaps = [0i16; 2];
    for (slot, player) in handicaps.iter_mut().zip(m.players()) {
        *slot=sqlx::query_scalar("SELECT playing_handicap FROM round_handicap_snapshots WHERE round_id=$1 AND player_id=$2").bind(c.id).bind(player).fetch_optional(&mut **tx).await?.map(Ok).unwrap_or_else(|| if c.status==RoundStatus::Draft {Ok(0)}else{Err(Error::Conflict("missing preserved handicap snapshot"))})?;
    }
    let indexes = sqlx::query_scalar::<_, i16>(
        "SELECT stroke_index FROM holes WHERE tee_id=$1 ORDER BY hole_number",
    )
    .bind(c.tee_id)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|s| u8::try_from(s).map_err(|_| Error::Conflict("invalid stroke index")))
    .collect::<Result<Vec<_>, _>>()?;
    if c.status != RoundStatus::Draft {
        let mut sorted = indexes.clone();
        sorted.sort();
        if sorted != (1..=18).collect::<Vec<u8>>() {
            return Err(Error::Conflict("invalid preserved hole configuration"));
        }
    }
    Ok((
        HandicapAllocation::new(
            if c.handicap_enabled {
                MatchMode::Net
            } else {
                MatchMode::Gross
            },
            handicaps,
        ),
        indexes,
    ))
}
pub(super) async fn write_context(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    actor: Uuid,
    round: Uuid,
) -> Result<(), Error> {
    sqlx::query("SELECT set_config('app.match_actor',$1::text,true),set_config('app.match_session',$2::text,true),set_config('app.score_mutation_round_id',$3::text,true)").bind(actor).bind(session).bind(round).execute(&mut **tx).await?;
    Ok(())
}
