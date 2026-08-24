mod load;
mod opening;

use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        models::{OpenRoundResult, PairingValidation},
        round_lifecycle::validate,
        scoring::ScoringError,
    },
    repositories::tournament_authorization::{self, AuthorizationError},
};

#[derive(Debug, Error)]
pub enum OpenRoundError {
    #[error("round not found")]
    NotFound,
    #[error("round is not ready to open")]
    NotReady(PairingValidation),
    #[error("round handicap calculation failed")]
    Scoring(#[from] ScoringError),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn pairing_validation(
    pool: &PgPool,
    round_id: Uuid,
) -> Result<Option<PairingValidation>, sqlx::Error> {
    let mut connection = pool.acquire().await?;
    Ok(load::round(&mut connection, round_id, false)
        .await?
        .map(|loaded| validate(&loaded.facts)))
}

pub async fn pairing_validation_for_member(
    pool: &PgPool,
    user_id: Uuid,
    round_id: Uuid,
) -> Result<PairingValidation, AuthorizationError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    tournament_authorization::require_round_member_read(&mut transaction, user_id, round_id)
        .await?;
    let validation = load::round(&mut transaction, round_id, false)
        .await?
        .map(|loaded| validate(&loaded.facts))
        .ok_or(AuthorizationError::NotFound)?;
    transaction.commit().await?;
    Ok(validation)
}

pub async fn open(pool: &PgPool, round_id: Uuid) -> Result<OpenRoundResult, OpenRoundError> {
    let mut transaction = pool.begin().await?;
    let result = opening::in_transaction(&mut transaction, round_id).await?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn open_authorized(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
) -> Result<OpenRoundResult, OpenRoundError> {
    let mut transaction = pool.begin().await?;
    tournament_authorization::require_round_admin(&mut transaction, session_id, round_id).await?;
    let result = opening::in_transaction(&mut transaction, round_id).await?;
    transaction.commit().await?;
    Ok(result)
}
