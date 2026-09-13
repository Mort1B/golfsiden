use super::*;
use serde::Serialize;
#[derive(Serialize)]
pub struct Completion {
    pub format: ScoringFormat,
    pub round_id: Uuid,
    pub status: RoundStatus,
    pub matches: Vec<Progress>,
    pub ready_to_complete: Option<bool>,
    pub ready_to_lock: Option<bool>,
}
#[derive(Serialize)]
pub struct Progress {
    pub match_id: Uuid,
    pub terminal: Option<bool>,
    pub confirmed: Option<bool>,
}
pub async fn get(pool: &PgPool, session: Uuid, round: Uuid) -> Result<Completion, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    let c = context(&mut tx, round, false).await?;
    let (_, role) = member(&mut tx, session, c.tournament_id).await?;
    let hidden = reads::metadata(&c, role).mode
        == crate::domain::score_visibility::VisibilityMode::FrontNine;
    let rows = sqlx::query_as::<_, (Uuid, bool, bool)>(
        "SELECT id,terminal,confirmed FROM singles_matches WHERE round_id=$1 ORDER BY id",
    )
    .bind(round)
    .fetch_all(&mut *tx)
    .await?;
    let ready = !rows.is_empty()
        && rows
            .iter()
            .all(|(_, terminal, confirmed)| *terminal && *confirmed);
    let matches = rows
        .into_iter()
        .map(|(match_id, terminal, confirmed)| Progress {
            match_id,
            terminal: (!hidden).then_some(terminal),
            confirmed: (!hidden).then_some(confirmed),
        })
        .collect();
    tx.commit().await?;
    Ok(Completion {
        format: ScoringFormat::SinglesMatchPlay,
        round_id: round,
        status: c.status,
        matches,
        ready_to_complete: (!hidden).then_some(ready && c.status == RoundStatus::Open),
        ready_to_lock: (!hidden).then_some(ready && c.status == RoundStatus::Completed),
    })
}
// Called after the established exact-admin authorization and parent-round lock.
pub async fn transition(
    tx: &mut Transaction<'_, Postgres>,
    round: Uuid,
    action: crate::domain::round_completion::TransitionAction,
) -> Result<crate::domain::models::Round, crate::repositories::round_completion::RoundCompletionError>
{
    use crate::{
        domain::{
            models::Round,
            round_completion::{TransitionAction, TransitionBlocker},
        },
        repositories::round_completion::RoundCompletionError,
    };
    let (source, target, setting) = match action {
        TransitionAction::Complete => (
            RoundStatus::Open,
            RoundStatus::Completed,
            "app.round_completion_id",
        ),
        TransitionAction::Lock => (
            RoundStatus::Completed,
            RoundStatus::Locked,
            "app.round_lock_id",
        ),
    };
    let status =
        sqlx::query_scalar::<_, RoundStatus>("SELECT status FROM rounds WHERE id=$1 FOR UPDATE")
            .bind(round)
            .fetch_one(&mut **tx)
            .await?;
    if status != source {
        return Err(RoundCompletionError::Blocked {
            action,
            blocker: TransitionBlocker::InvalidSourceState,
        });
    }
    let ready = sqlx::query_scalar::<_, bool>("SELECT round_scorecards_ready($1)")
        .bind(round)
        .fetch_one(&mut **tx)
        .await?;
    if !ready {
        return Err(RoundCompletionError::Blocked {
            action,
            blocker: TransitionBlocker::UnconfirmedScorecards,
        });
    }
    sqlx::query("SELECT set_config($1,$2::text,true)")
        .bind(setting)
        .bind(round)
        .execute(&mut **tx)
        .await?;
    Ok(
        sqlx::query_as::<_, Round>("UPDATE rounds SET status=$2 WHERE id=$1 RETURNING *")
            .bind(round)
            .bind(target)
            .fetch_one(&mut **tx)
            .await?,
    )
}
