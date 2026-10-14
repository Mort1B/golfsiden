mod load;
mod replace;
pub mod types;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::round_pairings::ReplacementCommand,
    repositories::{
        round_pairings::types::RoundPairings,
        session_read::{self, SessionReadError},
        tournament_authorization::{self, AuthorizationError},
    },
};

#[derive(Debug, Error)]
pub enum RoundPairingsError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("round must be draft")]
    NotDraft,
    #[error("round pairings have changed")]
    Stale,
    #[error("pairing request conflicts with stored identities")]
    IdentityConflict,
    #[error("submitted members must be active eligible entrants for this format")]
    InvalidRoster,
    #[error("legacy individual groups require an exact conversion mapping")]
    LegacyMappingRequired,
    #[error("legacy conversion does not preserve the stored group")]
    InvalidLegacyConversion,
    #[error("a scheduled scramble team requires an explicit identical flight transfer")]
    InvalidScheduleTransfer,
    #[error("a referenced shared-result team cannot be removed")]
    ReferencedTeam,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn get_for_session(
    pool: &PgPool,
    session: Uuid,
    round_id: Uuid,
) -> Result<RoundPairings, SessionReadError> {
    session_read::retry(|| async {
        let (mut tx, user) = session_read::begin(pool, session).await?;
        tournament_authorization::require_round_member_read(&mut tx, user, round_id).await?;
        let round = load::round(&mut tx, round_id)
            .await?
            .ok_or(AuthorizationError::NotFound)?;
        let model = load::model(&mut tx, round).await?;
        session_read::finish(tx, session).await?;
        Ok(model)
    })
    .await
}

pub async fn preflight_admin(
    pool: &PgPool,
    user_id: Uuid,
    round_id: Uuid,
) -> Result<DateTime<Utc>, RoundPairingsError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    let round = load::round(&mut transaction, round_id)
        .await?
        .ok_or(AuthorizationError::NotFound)?;
    tournament_authorization::require_tournament_admin_read_in_transaction(
        &mut transaction,
        user_id,
        round.tournament_id,
    )
    .await?;
    if round.status != crate::domain::models::RoundStatus::Draft {
        return Err(RoundPairingsError::NotDraft);
    }
    transaction.commit().await?;
    Ok(round.updated_at)
}

pub async fn replace(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    expected_updated_at: DateTime<Utc>,
    command: &ReplacementCommand,
) -> Result<RoundPairings, RoundPairingsError> {
    replace::execute(pool, session_id, round_id, expected_updated_at, command).await
}
