mod validation;
mod writes;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{models::RoundStatus, round_pairings::ReplacementCommand},
    repositories::{
        round_pairings::{RoundPairingsError, load, types::RoundPairings},
        tournament_authorization,
    },
};

pub(super) async fn execute(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    expected_updated_at: DateTime<Utc>,
    command: &ReplacementCommand,
) -> Result<RoundPairings, RoundPairingsError> {
    let mut tx = pool.begin().await?;
    tournament_authorization::require_round_admin(&mut tx, session_id, round_id).await?;
    let round = load::round(&mut tx, round_id)
        .await?
        .ok_or(tournament_authorization::AuthorizationError::NotFound)?;
    if round.status != RoundStatus::Draft {
        return Err(RoundPairingsError::NotDraft);
    }
    if round.updated_at != expected_updated_at {
        return Err(RoundPairingsError::Stale);
    }
    validation::identities_and_roster(&mut tx, &round, command).await?;
    let legacy = validation::legacy_and_schedules(&mut tx, &round, command).await?;
    validation::reject_referenced_team_deletions(&mut tx, round_id, command).await?;

    writes::replace_memberships_and_groups(&mut tx, &round, command, legacy).await?;
    sqlx::query("UPDATE rounds SET updated_at = clock_timestamp() WHERE id = $1")
        .bind(round_id)
        .execute(&mut *tx)
        .await?;
    let updated = load::round(&mut tx, round_id)
        .await?
        .ok_or(tournament_authorization::AuthorizationError::NotFound)?;
    let model = load::model(&mut tx, updated).await?;
    tx.commit().await?;
    Ok(model)
}
