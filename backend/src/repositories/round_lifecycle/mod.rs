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
    repositories::{
        auth,
        session_read::{self, SessionReadError},
        tournament_authorization::{self, AuthorizationError},
    },
};

#[derive(Debug, Error)]
pub enum OpenRoundError {
    #[error("round not found")]
    NotFound,
    #[error("round is not ready to open")]
    NotReady(Box<PairingValidation>),
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
    let mut result = load::round(&mut connection, round_id, false)
        .await?
        .map(|loaded| validate(&loaded.facts));
    if let Some(validation) = &mut result {
        crate::repositories::match_play::setup::add_readiness(&mut connection, validation).await?;
    }
    Ok(result)
}

pub async fn pairing_validation_for_session(
    pool: &PgPool,
    session: Uuid,
    round_id: Uuid,
) -> Result<PairingValidation, SessionReadError> {
    session_read::retry(|| async {
        let (mut transaction, user) = session_read::begin(pool, session).await?;
        tournament_authorization::require_round_member_read(&mut transaction, user, round_id)
            .await?;
        let mut validation = load::round(&mut transaction, round_id, false)
            .await?
            .map(|loaded| validate(&loaded.facts))
            .ok_or(AuthorizationError::NotFound)?;
        crate::repositories::match_play::setup::add_readiness(&mut transaction, &mut validation)
            .await?;
        session_read::finish(transaction, session).await?;
        Ok(validation)
    })
    .await
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
    auth::lock_active_session(&mut transaction, session_id)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    transaction.commit().await?;
    Ok(result)
}
