use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use thiserror::Error;
use uuid::Uuid;

use super::tournament_authorization::{self, AuthorizationError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct FinalRoundVisibility {
    pub tournament_id: Uuid,
    pub back_nine_hidden: bool,
    pub visibility_updated_at: DateTime<Utc>,
}

pub struct UpdateFinalRoundVisibilityResult {
    pub visibility: FinalRoundVisibility,
    pub changed: bool,
}

#[derive(Debug, Error)]
pub enum FinalRoundVisibilityError {
    #[error("resource not found")]
    NotFound,
    #[error("final-round visibility has changed")]
    Stale,
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

const COLUMNS: &str =
    "id AS tournament_id, final_round_back_nine_hidden AS back_nine_hidden, visibility_updated_at";

pub async fn get_for_admin(
    pool: &PgPool,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<FinalRoundVisibility, FinalRoundVisibilityError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    let visibility = sqlx::query_as::<_, FinalRoundVisibility>(&format!(
        "SELECT {COLUMNS} FROM tournaments WHERE id = $1"
    ))
    .bind(tournament_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(FinalRoundVisibilityError::NotFound)?;
    tournament_authorization::require_tournament_admin_read_in_transaction(
        &mut transaction,
        user_id,
        tournament_id,
    )
    .await?;
    transaction.commit().await?;
    Ok(visibility)
}

pub async fn update_authorized(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    back_nine_hidden: bool,
    expected_visibility_updated_at: DateTime<Utc>,
) -> Result<UpdateFinalRoundVisibilityResult, FinalRoundVisibilityError> {
    preflight_admin(pool, session_id, tournament_id).await?;

    let mut transaction = pool.begin().await?;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM rounds WHERE tournament_id = $1 ORDER BY id FOR UPDATE",
    )
    .bind(tournament_id)
    .fetch_all(&mut *transaction)
    .await?;
    tournament_authorization::require_tournament_admin(&mut transaction, session_id, tournament_id)
        .await?;
    let visibility = sqlx::query_as::<_, FinalRoundVisibility>(&format!(
        "SELECT {COLUMNS} FROM tournaments WHERE id = $1 FOR UPDATE"
    ))
    .bind(tournament_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(FinalRoundVisibilityError::NotFound)?;

    if visibility.visibility_updated_at != expected_visibility_updated_at {
        return Err(FinalRoundVisibilityError::Stale);
    }
    if visibility.back_nine_hidden == back_nine_hidden {
        transaction.commit().await?;
        return Ok(UpdateFinalRoundVisibilityResult {
            visibility,
            changed: false,
        });
    }

    sqlx::query(
        "SELECT
           set_config('app.final_round_visibility_tournament_id', $1::text, true),
           set_config('app.final_round_visibility_session_id', $2::text, true)",
    )
    .bind(tournament_id)
    .bind(session_id)
    .execute(&mut *transaction)
    .await?;
    let visibility = sqlx::query_as::<_, FinalRoundVisibility>(&format!(
        "UPDATE tournaments SET final_round_back_nine_hidden = $2
         WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(tournament_id)
    .bind(back_nine_hidden)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(UpdateFinalRoundVisibilityResult {
        visibility,
        changed: true,
    })
}

async fn preflight_admin(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
) -> Result<(), FinalRoundVisibilityError> {
    let likely_admin = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1
           FROM user_sessions AS session
           JOIN tournament_memberships AS membership ON membership.user_id = session.user_id
           WHERE session.id = $1
             AND session.revoked_at IS NULL
             AND session.expires_at > now()
             AND membership.tournament_id = $2
             AND membership.role = 'admin'
         )",
    )
    .bind(session_id)
    .bind(tournament_id)
    .fetch_one(pool)
    .await?;
    if likely_admin {
        return Ok(());
    }

    let mut authorization = pool.begin().await?;
    tournament_authorization::require_tournament_admin(
        &mut authorization,
        session_id,
        tournament_id,
    )
    .await?;
    authorization.rollback().await?;
    Ok(())
}
