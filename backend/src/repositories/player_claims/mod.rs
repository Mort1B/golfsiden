mod admin;
mod public;

pub use admin::{AccountState, Grant, create, list, reissue, revoke, withdraw};
pub use public::{ClaimPreview, RegisterParams, preview, register};

use crate::repositories::{
    auth,
    tournament_authorization::{self, AuthorizationError},
};
use sqlx::{Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ClaimError {
    #[error("claim is invalid")]
    Invalid,
    #[error("claim is unavailable")]
    Unavailable,
    #[error("player already has an account")]
    HasAccount,
    #[error("player is withdrawn or inactive")]
    Withdrawn,
    #[error("tournament is closed")]
    Closed,
    #[error("player is an administrator")]
    Admin,
    #[error("remove draft assignments first")]
    DraftAssignment,
    #[error("lock participating rounds first")]
    LiveRound,
    #[error("player participation changed concurrently")]
    Busy,
    #[error("username is already registered")]
    Username,
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[source] sqlx::Error),
}
impl From<sqlx::Error> for ClaimError {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &error {
            if db.code().as_deref() == Some("55P03") {
                return Self::Busy;
            }
            match db.constraint() {
                Some("users_username_normalized_idx") => return Self::Username,
                Some("users_player_id_key") => return Self::HasAccount,
                Some("tournament_closed_to_joining") => return Self::Closed,
                Some("player_is_admin") => return Self::Admin,
                Some("player_assigned_draft") => return Self::DraftAssignment,
                Some("player_round_in_progress") => return Self::LiveRound,
                Some("player_withdrawn") => return Self::Withdrawn,
                _ => {}
            }
        }
        Self::Database(error)
    }
}
async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    tournament: Uuid,
) -> Result<Uuid, ClaimError> {
    Ok(tournament_authorization::require_tournament_admin(tx, session, tournament).await?)
}
async fn recheck_session(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
) -> Result<(), ClaimError> {
    if auth::lock_active_session(tx, session).await?.is_none() {
        return Err(AuthorizationError::Unauthenticated.into());
    }
    // Re-read the clock after any lock acquisition wait.
    let alive: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_sessions WHERE id=$1 AND revoked_at IS NULL AND expires_at>clock_timestamp())")
        .bind(session).fetch_one(&mut **tx).await?;
    if !alive {
        return Err(AuthorizationError::Unauthenticated.into());
    }
    Ok(())
}
async fn lock_player(
    tx: &mut Transaction<'_, Postgres>,
    tournament: Uuid,
    player: Uuid,
) -> Result<(), ClaimError> {
    let active: Option<bool> =
        sqlx::query_scalar("SELECT active FROM players WHERE id=$1 FOR UPDATE")
            .bind(player)
            .fetch_optional(&mut **tx)
            .await?;
    let enrolled: Option<bool> = sqlx::query_scalar("SELECT status='active' FROM tournament_players WHERE tournament_id=$1 AND player_id=$2 FOR UPDATE")
        .bind(tournament).bind(player).fetch_optional(&mut **tx).await?;
    if enrolled.is_none() {
        return Err(AuthorizationError::NotFound.into());
    }
    if active != Some(true) || enrolled != Some(true) {
        return Err(ClaimError::Withdrawn);
    }
    Ok(())
}
async fn require_unlinked(
    tx: &mut Transaction<'_, Postgres>,
    player: Uuid,
) -> Result<(), ClaimError> {
    let linked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE player_id=$1)")
        .bind(player)
        .fetch_one(&mut **tx)
        .await?;
    if linked {
        return Err(ClaimError::HasAccount);
    }
    Ok(())
}
