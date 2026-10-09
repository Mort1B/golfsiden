mod config;
mod lifecycle;
mod lineups;
mod models;
pub mod results;
mod sources;

use crate::repositories::{
    auth,
    tournament_authorization::{self, AuthorizationError},
};
pub use config::{configure, enter, game, set_deadline};
pub use lineups::{read_round, save};
pub use models::*;
pub use sources::{dispose, source};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("Fantasy is unavailable")]
    Unavailable,
    #[error("selection window is closed")]
    Closed,
    #[error("invalid Fantasy request")]
    Invalid,
    #[error("revision or request conflicts; refresh and retry")]
    Conflict,
    #[error("unaccepted request has a stale expected revision")]
    RevisionConflict,
    #[error("database operation failed")]
    Database(#[source] sqlx::Error),
}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        if let sqlx::Error::Database(e) = &value
            && matches!(
                e.code().as_deref(),
                Some("23503" | "23505" | "23514" | "55P03" | "40P01" | "40001")
            )
        {
            return Self::Conflict;
        }
        Self::Database(value)
    }
}
async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    t: Uuid,
    admin: bool,
) -> Result<Uuid, Error> {
    let principal = auth::lock_active_session(tx, session)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    tournament_authorization::require_tournament_member_read(tx, principal.user_id, t).await?;
    if admin {
        tournament_authorization::require_tournament_admin_read_in_transaction(
            tx,
            principal.user_id,
            t,
        )
        .await?;
    }
    Ok(principal.user_id)
}
async fn recheck(tx: &mut Transaction<'_, Postgres>, session: Uuid) -> Result<(), Error> {
    if auth::lock_active_session(tx, session).await?.is_none() {
        return Err(AuthorizationError::Unauthenticated.into());
    }
    let alive:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_sessions WHERE id=$1 AND revoked_at IS NULL AND expires_at>clock_timestamp())").bind(session).fetch_one(&mut **tx).await?;
    if !alive {
        return Err(AuthorizationError::Unauthenticated.into());
    }
    Ok(())
}
// All round locks, in the same stable order as withdrawal. This also serializes
// deadline histories and avoids materializing a later round before its source.
async fn begin(
    pool: &PgPool,
    session: Uuid,
    t: Uuid,
    admin: bool,
) -> Result<(Transaction<'_, Postgres>, Uuid), Error> {
    let mut pre = pool.begin().await?;
    authorize(&mut pre, session, t, admin).await?;
    pre.rollback().await?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE")
        .bind(t)
        .fetch_all(&mut *tx)
        .await?;
    let actor = authorize(&mut tx, session, t, admin).await?;
    sqlx::query(
        "SELECT set_config('app.fantasy_session',$1,true),set_config('app.fantasy_actor',$2,true)",
    )
    .bind(session.to_string())
    .bind(actor.to_string())
    .execute(&mut *tx)
    .await?;
    Ok((tx, actor))
}
async fn enabled(tx: &mut Transaction<'_, Postgres>, t: Uuid) -> Result<(), Error> {
    let yes: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM fantasy_games WHERE tournament_id=$1 FOR UPDATE")
            .bind(t)
            .fetch_optional(&mut **tx)
            .await?;
    if yes != Some(true) {
        return Err(Error::Unavailable);
    }
    Ok(())
}
