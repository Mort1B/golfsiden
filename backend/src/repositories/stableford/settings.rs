use crate::{
    domain::models::{Round, RoundStatus, ScoringFormat},
    repositories::{
        auth,
        tournament_authorization::{self, AuthorizationError},
    },
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("Stableford settings require a draft round")]
    NotDraft,
    #[error("round configuration has changed")]
    Stale,
    #[error("allowance must be from 0 to 100")]
    InvalidAllowance,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}
pub async fn update(
    pool: &PgPool,
    session: Uuid,
    round: Uuid,
    expected: DateTime<Utc>,
    enabled: bool,
    allowance: i16,
) -> Result<Round, SettingsError> {
    if !(0..=100).contains(&allowance) {
        return Err(SettingsError::InvalidAllowance);
    }
    let mut tx = pool.begin().await?;
    tournament_authorization::require_round_admin(&mut tx, session, round).await?;
    let current = sqlx::query_as::<_, Round>("SELECT * FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(round)
        .fetch_one(&mut *tx)
        .await?;
    if current.status != RoundStatus::Draft
        || current.scoring_format != ScoringFormat::IndividualStableford
    {
        return Err(SettingsError::NotDraft);
    }
    if current.updated_at != expected {
        return Err(SettingsError::Stale);
    }
    let result = if current.handicap_enabled == enabled
        && current.handicap_allowance_percent == allowance
    {
        current
    } else {
        sqlx::query_as::<_,Round>("UPDATE rounds SET handicap_enabled=$2,handicap_allowance_percent=$3 WHERE id=$1 RETURNING *").bind(round).bind(enabled).bind(allowance).fetch_one(&mut *tx).await?
    };
    auth::lock_active_session(&mut tx, session)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    tx.commit().await?;
    Ok(result)
}
