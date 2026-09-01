use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{domain::models::TournamentRole, repositories::auth};

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("resource not found")]
    NotFound,
    #[error("session is not authenticated")]
    Unauthenticated,
    #[error("session is not authorized for this tournament")]
    Forbidden,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn require_tournament_member_read(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<(), AuthorizationError> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tournaments WHERE id = $1)")
            .bind(tournament_id)
            .fetch_one(&mut **transaction)
            .await?;
    if !exists {
        return Err(AuthorizationError::NotFound);
    }
    require_membership_read(transaction, user_id, tournament_id).await
}

pub async fn require_tournament_admin_read(
    pool: &PgPool,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<(), AuthorizationError> {
    let mut transaction = pool.begin().await?;
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tournaments WHERE id = $1)")
            .bind(tournament_id)
            .fetch_one(&mut *transaction)
            .await?;
    if !exists {
        return Err(AuthorizationError::NotFound);
    }
    let role = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2
         FOR SHARE",
    )
    .bind(tournament_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
    .await?;
    if role != Some(TournamentRole::Admin) {
        return Err(AuthorizationError::Forbidden);
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn require_tournament_admin_read_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<(), AuthorizationError> {
    let role = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2
         FOR SHARE",
    )
    .bind(tournament_id)
    .bind(user_id)
    .fetch_optional(&mut **transaction)
    .await?;
    if role != Some(TournamentRole::Admin) {
        return Err(AuthorizationError::Forbidden);
    }
    Ok(())
}

pub async fn require_round_member_read(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    round_id: Uuid,
) -> Result<Uuid, AuthorizationError> {
    let tournament_id =
        sqlx::query_scalar::<_, Uuid>("SELECT tournament_id FROM rounds WHERE id = $1")
            .bind(round_id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or(AuthorizationError::NotFound)?;
    require_membership_read(transaction, user_id, tournament_id).await?;
    Ok(tournament_id)
}

async fn require_membership_read(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<(), AuthorizationError> {
    let role = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2
         FOR SHARE",
    )
    .bind(tournament_id)
    .bind(user_id)
    .fetch_optional(&mut **transaction)
    .await?;
    if role.is_none() {
        return Err(AuthorizationError::Forbidden);
    }
    Ok(())
}

pub async fn require_tournament_admin(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    tournament_id: Uuid,
) -> Result<Uuid, AuthorizationError> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tournaments WHERE id = $1)")
            .bind(tournament_id)
            .fetch_one(&mut **transaction)
            .await?;
    if !exists {
        return Err(AuthorizationError::NotFound);
    }
    let principal = auth::lock_active_session(transaction, session_id)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    let role = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2
         FOR SHARE",
    )
    .bind(tournament_id)
    .bind(principal.user_id)
    .fetch_optional(&mut **transaction)
    .await?;
    if role != Some(TournamentRole::Admin) {
        return Err(AuthorizationError::Forbidden);
    }
    Ok(principal.user_id)
}

pub async fn require_round_admin(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    round_id: Uuid,
) -> Result<(Uuid, Uuid), AuthorizationError> {
    let tournament_id =
        sqlx::query_scalar::<_, Uuid>("SELECT tournament_id FROM rounds WHERE id = $1 FOR UPDATE")
            .bind(round_id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or(AuthorizationError::NotFound)?;
    let actor = require_tournament_admin(transaction, session_id, tournament_id).await?;
    Ok((tournament_id, actor))
}

pub async fn require_team_admin(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    team_id: Uuid,
) -> Result<(Uuid, Uuid), AuthorizationError> {
    let tournament_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT t.tournament_id FROM teams t
         JOIN rounds r ON r.id = t.round_id
         WHERE t.id = $1 FOR UPDATE OF r",
    )
    .bind(team_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(AuthorizationError::NotFound)?;
    let actor = require_tournament_admin(transaction, session_id, tournament_id).await?;
    Ok((tournament_id, actor))
}
