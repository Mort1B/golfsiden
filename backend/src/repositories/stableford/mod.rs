mod mutations;
mod rows;
pub mod settings;
use crate::{
    domain::{
        models::{RoundStatus, ScoringFormat, TournamentRole},
        score_visibility::{VisibilityFacts, unrestricted, visibility},
        scorecards::ScoreOwner,
        stableford::card::{Card, HoleSource, Partner, project},
    },
    repositories::{
        auth, score_authorization,
        scorecards::{ScorecardConflict, ScorecardError},
    },
};
pub use mutations::{SaveInput, confirm, save_conditional};
use rows::{Context, InputRow};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

pub async fn get(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    player_id: Uuid,
    scoring: bool,
) -> Result<Card, ScorecardError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    let context = context(&mut tx, round_id, false).await?;
    let principal = auth::lock_active_session(&mut tx, session_id)
        .await?
        .ok_or(ScorecardError::Unauthenticated)?;
    let role = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR SHARE",
    )
    .bind(context.tournament_id)
    .bind(principal.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ScorecardError::Forbidden)?;
    require_format(&context)?;
    let player = player(&mut tx, round_id, player_id).await?;
    let metadata = if scoring {
        editable(&context)?;
        authorize_player(&mut tx, session_id, &context, player_id).await?;
        unrestricted()
    } else {
        visibility(VisibilityFacts {
            role,
            is_final_round: context.round_number == context.tournament_round_count,
            status: context.status,
            number_of_holes: 18,
            back_nine_hidden: context.final_round_back_nine_hidden,
        })
    };
    let card = build(&mut tx, &context, player, metadata).await?;
    tx.commit().await?;
    Ok(card)
}
async fn context(
    tx: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
    lock: bool,
) -> Result<Context, ScorecardError> {
    let query = if lock {
        "SELECT r.id,r.tournament_id,r.tee_id,r.status,r.scoring_format,r.round_number,t.number_of_rounds AS tournament_round_count,t.final_round_back_nine_hidden FROM rounds r JOIN tournaments t ON t.id=r.tournament_id WHERE r.id=$1 FOR UPDATE OF r"
    } else {
        "SELECT r.id,r.tournament_id,r.tee_id,r.status,r.scoring_format,r.round_number,t.number_of_rounds AS tournament_round_count,t.final_round_back_nine_hidden FROM rounds r JOIN tournaments t ON t.id=r.tournament_id WHERE r.id=$1"
    };
    let context = sqlx::query_as::<_, Context>(query)
        .bind(round_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(ScorecardError::NotFound)?;
    Ok(context)
}
fn require_format(context: &Context) -> Result<(), ScorecardError> {
    if context.scoring_format == ScoringFormat::IndividualStableford {
        Ok(())
    } else {
        Err(ScorecardError::Conflict(
            ScorecardConflict::OwnerFormatMismatch,
        ))
    }
}
async fn player(
    connection: &mut PgConnection,
    round_id: Uuid,
    player_id: Uuid,
) -> Result<Partner, ScorecardError> {
    sqlx::query_as::<_,Partner>("SELECT rhs.player_id,p.display_name,rhs.playing_handicap FROM round_handicap_snapshots rhs JOIN players p ON p.id=rhs.player_id WHERE rhs.round_id=$1 AND rhs.player_id=$2")
        .bind(round_id).bind(player_id).fetch_optional(connection).await?.ok_or(ScorecardError::NotFound)
}
async fn authorize_player(
    tx: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    context: &Context,
    player_id: Uuid,
) -> Result<Uuid, ScorecardError> {
    score_authorization::authorize_mutation(
        tx,
        session_id,
        context.id,
        context.scoring_format,
        ScoreOwner::Player { id: player_id },
    )
    .await
    .map_err(map_authorization)
}
fn map_authorization(error: score_authorization::ScoreAuthorizationError) -> ScorecardError {
    use score_authorization::ScoreAuthorizationError as E;
    match error {
        E::NotFound => ScorecardError::NotFound,
        E::Unauthenticated => ScorecardError::Unauthenticated,
        E::Forbidden => ScorecardError::Forbidden,
        E::Database(e) => ScorecardError::Database(e),
    }
}
fn editable(context: &Context) -> Result<(), ScorecardError> {
    require_format(context)?;
    if matches!(context.status, RoundStatus::Open | RoundStatus::Completed) {
        Ok(())
    } else {
        Err(ScorecardError::Conflict(
            ScorecardConflict::RoundNotEditable,
        ))
    }
}
async fn build(
    connection: &mut PgConnection,
    context: &Context,
    player: Partner,
    metadata: crate::domain::score_visibility::VisibilityMetadata,
) -> Result<Card, ScorecardError> {
    let sources = sqlx::query_as::<_, HoleSource>("SELECT id AS hole_id,hole_number,par,stroke_index FROM holes WHERE tee_id=$1 ORDER BY hole_number,id").bind(context.tee_id).fetch_all(&mut *connection).await?;

    let entries = sqlx::query_as::<_, InputRow>("SELECT id,revision,round_id,hole_id,player_id,gross_strokes,submitted_by,submitted_at,updated_at FROM stableford_inputs WHERE round_id=$1 AND player_id=$2 ORDER BY hole_id,player_id")
        .bind(context.id).bind(player.player_id).fetch_all(&mut *connection).await?.into_iter().map(InputRow::entry).collect::<Result<Vec<_>,_>>()?;
    let confirmation = sqlx::query_as::<_, (Uuid,chrono::DateTime<chrono::Utc>)>("SELECT confirmed_by,confirmed_at FROM scorecard_confirmations WHERE round_id=$1 AND player_id=$2").bind(context.id).bind(player.player_id).fetch_optional(connection).await?;
    Ok(project(
        Card {
            format: ScoringFormat::IndividualStableford,
            round_id: context.id,
            owner: ScoreOwner::Player {
                id: player.player_id,
            },
            owner_name: player.display_name,
            playing_handicap: player.playing_handicap,
            holes: Vec::new(),
            values: None,
            holes_scored: 0,
            number_of_holes: 18,
            visible_hole_count: 18,
            complete: None,
            confirmed: Some(confirmation.is_some()),
            confirmed_at: confirmation.map(|c| c.1),
            visibility: metadata,
        },
        sources,
        entries,
    )?)
}
