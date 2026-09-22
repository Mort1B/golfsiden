use super::*;
use crate::domain::match_play::table::{AwardFact, Participant, TableEntry, assemble};
use crate::domain::{
    match_play::{
        MatchFinish,
        commands::{Event, visible_event},
    },
    score_visibility::{
        VisibilityFacts, VisibilityMetadata, VisibilityMode, unrestricted, visibility,
    },
    scorecards::ScoreRevision,
};
use serde::Serialize;
#[derive(Serialize, sqlx::FromRow)]
pub struct OpponentView {
    pub player_id: Uuid,
    pub display_name: String,
    pub playing_handicap: Option<i16>,
}
#[derive(Serialize, sqlx::FromRow)]
pub struct Note {
    pub player_id: Uuid,
    pub hole_number: i16,
    pub gross_strokes: Option<i16>,
}
#[derive(Serialize, sqlx::FromRow)]
pub struct Hole {
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
}
#[derive(Serialize)]
pub struct Card {
    pub format: ScoringFormat,
    pub match_id: Uuid,
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub round_status: RoundStatus,
    pub mode: &'static str,
    pub opponents: Vec<OpponentView>,
    pub relative_handicaps: [i32; 2],
    pub holes: Vec<Hole>,
    pub notes: Vec<Note>,
    pub events: Vec<Event>,
    pub resolved_holes: u8,
    pub lead: i8,
    pub finish: Option<MatchFinish>,
    pub confirmed: Option<bool>,
    pub correction_pending: Option<bool>,
    pub half_points: Option<[u32; 2]>,
    pub visibility: VisibilityMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<ScoreRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_events: Option<Vec<AcceptedEvent>>,
}
pub(super) fn metadata(c: &Context, role: TournamentRole) -> VisibilityMetadata {
    visibility(VisibilityFacts {
        role,
        is_final_round: c.round_number == c.number_of_rounds,
        status: c.status,
        number_of_holes: 18,
        back_nine_hidden: c.final_round_back_nine_hidden,
    })
}
pub async fn get(
    pool: &PgPool,
    session: Uuid,
    round: Uuid,
    id: Uuid,
    scoring: bool,
) -> Result<Card, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    let c = context(&mut tx, round, false).await?;
    let (_, role) = member(&mut tx, session, c.tournament_id).await?;
    let m = aggregate(&mut tx, round, id, false).await?;
    let visibility = if scoring {
        if c.status == RoundStatus::Draft
            || (c.status == RoundStatus::Locked && role != TournamentRole::Admin)
        {
            return Err(Error::Forbidden);
        }
        authority(&mut tx, session, &c, &m).await?;
        unrestricted()
    } else {
        metadata(&c, role)
    };
    let card = build(&mut tx, &c, &m, visibility, scoring).await?;
    tx.commit().await?;
    Ok(card)
}
pub(super) async fn build(
    tx: &mut Transaction<'_, Postgres>,
    c: &Context,
    m: &Aggregate,
    visibility: VisibilityMetadata,
    scoring: bool,
) -> Result<Card, Error> {
    let (allocation, indexes) = allocation(tx, c, m).await?;
    let full = m.events()?;
    let hidden = visibility.mode == VisibilityMode::FrontNine;
    let permitted = full
        .iter()
        .filter(|e| !hidden || visible_event(&e.event, 9))
        .cloned()
        .collect::<Vec<_>>();
    let state = validate_ledger(&permitted, m.players(), allocation, &indexes, true)
        .map_err(Error::Invalid)?;
    // Metadata that can change solely due to hidden activity never enters this projection.
    let confirmed = (!hidden).then_some(m.confirmed);
    let points = if confirmed == Some(true) {
        state
            .point_award(crate::domain::match_play::ConfirmationStatus::Confirmed)
            .map_err(|_| Error::Conflict("invalid confirmed match"))?
            .map(|p| [p.first.half_units(), p.second.half_units()])
    } else {
        None
    };
    let opponents=sqlx::query_as::<_,OpponentView>("SELECT p.id AS player_id,p.display_name,s.playing_handicap FROM players p LEFT JOIN round_handicap_snapshots s ON s.player_id=p.id AND s.round_id=$1 WHERE p.id=ANY($2) ORDER BY CASE WHEN p.id=$3 THEN 0 ELSE 1 END").bind(c.id).bind(m.players().to_vec()).bind(m.first_player_id).fetch_all(&mut **tx).await?;
    let limit = if hidden { 9i16 } else { 18 };
    let holes=sqlx::query_as::<_,Hole>("SELECT hole_number,par,stroke_index FROM holes WHERE tee_id=$1 AND hole_number<=$2 ORDER BY hole_number").bind(c.tee_id).bind(limit).fetch_all(&mut **tx).await?;
    let notes=sqlx::query_as::<_,Note>("SELECT player_id,hole_number,gross_strokes FROM singles_match_notes WHERE match_id=$1 AND hole_number<=$2 ORDER BY hole_number,player_id").bind(m.id).bind(limit).fetch_all(&mut **tx).await?;
    Ok(Card {
        format: ScoringFormat::SinglesMatchPlay,
        match_id: m.id,
        round_id: c.id,
        tournament_id: c.tournament_id,
        round_status: c.status,
        mode: if c.handicap_enabled { "net" } else { "gross" },
        opponents,
        relative_handicaps: allocation.relative_handicaps(),
        holes,
        notes,
        events: permitted.into_iter().map(|e| e.event).collect(),
        resolved_holes: state.resolved_holes(),
        lead: state.lead(),
        finish: state.finish(),
        confirmed,
        correction_pending: (!hidden).then_some(m.correction_pending),
        half_points: points,
        visibility,
        revision: if scoring {
            Some(
                ScoreRevision::from_database(m.revision)
                    .ok_or(Error::Conflict("invalid revision"))?,
            )
        } else {
            None
        },
        accepted_events: scoring.then_some(full),
    })
}
#[derive(Serialize)]
pub struct Listing {
    pub round_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_id: Option<Uuid>,
    pub matches: Vec<Card>,
    pub writable_match_ids: Vec<Uuid>,
}
pub async fn list(pool: &PgPool, session: Uuid, round: Uuid) -> Result<Listing, Error> {
    list_selected(pool, session, round, None).await
}
pub async fn list_for_player(
    pool: &PgPool,
    session: Uuid,
    round: Uuid,
    player: Uuid,
) -> Result<Listing, Error> {
    list_selected(pool, session, round, Some(player)).await
}
async fn list_selected(
    pool: &PgPool,
    session: Uuid,
    round: Uuid,
    player: Option<Uuid>,
) -> Result<Listing, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    let c = context(&mut tx, round, false).await?;
    let (_, role) = member(&mut tx, session, c.tournament_id).await?;
    let ids = match player {
        Some(player) => sqlx::query_scalar::<_, Uuid>("SELECT id FROM singles_matches WHERE round_id=$1 AND (first_player_id=$2 OR second_player_id=$2) ORDER BY first_player_id,second_player_id,id")
            .bind(round).bind(player).fetch_all(&mut *tx).await?,
        None => sqlx::query_scalar::<_, Uuid>("SELECT id FROM singles_matches WHERE round_id=$1 ORDER BY first_player_id,second_player_id,id")
            .bind(round).fetch_all(&mut *tx).await?,
    };
    let mut matches = Vec::new();
    let mut writable = Vec::new();
    // Deliberate N+1 for full card discovery, capped by 500 manual assignments per round.
    // Player filtering selects at most one card before construction; all reads reuse the same privacy projection.
    for id in ids {
        let m = aggregate(&mut tx, round, id, false).await?;
        match authority(&mut tx, session, &c, &m).await {
            Ok(_)
                if c.status != RoundStatus::Draft
                    && (c.status != RoundStatus::Locked || role == TournamentRole::Admin) =>
            {
                writable.push(id)
            }
            Ok(_) => {}
            Err(Error::Forbidden) => {}
            Err(e) => return Err(e),
        }
        matches.push(build(&mut tx, &c, &m, metadata(&c, role), false).await?);
    }
    tx.commit().await?;
    Ok(Listing {
        round_id: round,
        player_id: player,
        matches,
        writable_match_ids: writable,
    })
}
#[derive(Serialize)]
pub struct Table {
    pub tournament_id: Uuid,
    pub entries: Vec<TableEntry>,
}
pub async fn table(pool: &PgPool, session: Uuid, tournament: Uuid) -> Result<Table, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tournaments WHERE id=$1)")
            .bind(tournament)
            .fetch_one(&mut *tx)
            .await?;
    if !exists {
        return Err(Error::NotFound);
    }
    let (_, role) = member(&mut tx, session, tournament).await?;
    let participants=sqlx::query_as::<_,Participant>("SELECT tp.player_id,p.display_name FROM tournament_players tp JOIN players p ON p.id=tp.player_id WHERE tp.tournament_id=$1 ORDER BY tp.player_id").bind(tournament).fetch_all(&mut *tx).await?;
    let awards=sqlx::query_as::<_,AwardFact>("SELECT m.first_player_id,m.second_player_id,m.first_half_points,m.second_half_points FROM singles_matches m JOIN rounds r ON r.id=m.round_id JOIN tournaments t ON t.id=r.tournament_id WHERE m.tournament_id=$1 AND m.confirmed AND m.terminal AND ($2 OR NOT(t.final_round_back_nine_hidden AND r.round_number=t.number_of_rounds)) ORDER BY r.round_number,m.id").bind(tournament).bind(role==TournamentRole::Admin).fetch_all(&mut *tx).await?;
    let entries = assemble(participants, awards).map_err(Error::Invalid)?;
    tx.commit().await?;
    Ok(Table {
        tournament_id: tournament,
        entries,
    })
}
