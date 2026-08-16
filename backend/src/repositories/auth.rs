use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::SessionPrincipal;

#[derive(Debug, FromRow)]
pub struct LoginUser {
    pub id: Uuid,
    pub password_hash: Option<String>,
}

pub async fn find_login_user(
    pool: &PgPool,
    username: &str,
) -> Result<Option<LoginUser>, sqlx::Error> {
    sqlx::query_as("SELECT id, password_hash FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
}

pub async fn create_session(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> Result<SessionPrincipal, sqlx::Error> {
    let session_id = Uuid::new_v4();
    sqlx::query_as(
        "WITH inserted AS (
            INSERT INTO user_sessions (id, user_id, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, user_id, expires_at
         )
         SELECT inserted.id AS session_id, u.id AS user_id, u.username, u.display_name,
                u.role, u.player_id, inserted.expires_at
         FROM inserted JOIN users u ON u.id = inserted.user_id",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await
}

pub async fn create_session_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> Result<SessionPrincipal, sqlx::Error> {
    let session_id = Uuid::new_v4();
    sqlx::query_as(
        "WITH inserted AS (
            INSERT INTO user_sessions (id, user_id, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, user_id, expires_at
         )
         SELECT inserted.id AS session_id, u.id AS user_id, u.username, u.display_name,
                u.role, u.player_id, inserted.expires_at
         FROM inserted JOIN users u ON u.id = inserted.user_id",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(&mut **transaction)
    .await
}

pub async fn find_active_session(
    pool: &PgPool,
    token_hash: &[u8],
) -> Result<Option<SessionPrincipal>, sqlx::Error> {
    sqlx::query_as(
        "SELECT s.id AS session_id, u.id AS user_id, u.username, u.display_name, u.role,
                u.player_id, s.expires_at
         FROM user_sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now()",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn lock_active_session(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
) -> Result<Option<SessionPrincipal>, sqlx::Error> {
    sqlx::query_as(
        "SELECT s.id AS session_id, u.id AS user_id, u.username, u.display_name, u.role,
                u.player_id, s.expires_at
         FROM user_sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.id = $1 AND s.revoked_at IS NULL AND s.expires_at > now()
         FOR SHARE OF s, u",
    )
    .bind(session_id)
    .fetch_optional(&mut **transaction)
    .await
}

pub async fn lock_active_session_exclusive(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
) -> Result<Option<SessionPrincipal>, sqlx::Error> {
    sqlx::query_as(
        "SELECT s.id AS session_id, u.id AS user_id, u.username, u.display_name, u.role,
                u.player_id, s.expires_at
         FROM user_sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.id = $1 AND s.revoked_at IS NULL AND s.expires_at > now()
         FOR UPDATE OF s, u",
    )
    .bind(session_id)
    .fetch_optional(&mut **transaction)
    .await
}

pub async fn revoke_session(pool: &PgPool, session_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE user_sessions
         SET revoked_at = COALESCE(revoked_at, now())
         WHERE id = $1",
    )
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}
