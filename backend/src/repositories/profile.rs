use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use super::auth;

#[derive(Serialize, FromRow)]
pub struct Profile {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub version: i64,
    pub player_id: Option<Uuid>,
    pub handicap: Option<f64>,
    pub player_active: Option<bool>,
    pub player_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("authentication required")]
    Unauthenticated,
    #[error("profile changed; reload before editing")]
    Stale,
    #[error("current password is incorrect")]
    IncorrectPassword,
    #[error("inactive player handicap cannot be changed")]
    Inactive,
    #[error("handicap cannot be removed")]
    MissingHandicap,
    #[error("handicap changes require a reason")]
    MissingReason,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

async fn read(tx: &mut Transaction<'_, Postgres>, user: Uuid) -> Result<Profile, sqlx::Error> {
    sqlx::query_as("SELECT u.id AS user_id,u.username,u.display_name,u.profile_version AS version,
        u.player_id,p.current_handicap_index::float8 AS handicap,p.active AS player_active,
        p.updated_at AS player_updated_at FROM users u LEFT JOIN players p ON p.id=u.player_id WHERE u.id=$1")
        .bind(user).fetch_one(&mut **tx).await
}

pub async fn get(pool: &PgPool, session: Uuid) -> Result<Profile, ProfileError> {
    let mut tx = pool.begin().await?;
    let principal = auth::lock_active_session(&mut tx, session)
        .await?
        .ok_or(ProfileError::Unauthenticated)?;
    if let Some(player) = principal.player_id {
        sqlx::query("SELECT id FROM players WHERE id=$1 FOR SHARE")
            .bind(player)
            .execute(&mut *tx)
            .await?;
    }
    let profile = read(&mut tx, principal.user_id).await?;
    require_live(&mut tx, session).await?;
    tx.commit().await?;
    Ok(profile)
}

pub struct DetailsChange {
    pub version: i64,
    pub player_updated_at: Option<DateTime<Utc>>,
    pub display_name: String,
    pub handicap: Option<f64>,
    pub reason: String,
}

pub struct ProfileChangeResult {
    pub profile: Profile,
    pub tournaments: Vec<Uuid>,
}

pub async fn update_details(
    pool: &PgPool,
    session: Uuid,
    input: DetailsChange,
) -> Result<ProfileChangeResult, ProfileError> {
    let mut tx = pool.begin().await?;
    let principal = auth::lock_active_session_exclusive(&mut tx, session)
        .await?
        .ok_or(ProfileError::Unauthenticated)?;
    if let Some(player) = principal.player_id {
        sqlx::query("SELECT id FROM players WHERE id=$1 FOR UPDATE")
            .bind(player)
            .execute(&mut *tx)
            .await?;
    }
    let previous = read(&mut tx, principal.user_id).await?;
    require_live(&mut tx, session).await?;
    if previous.version != input.version || previous.player_updated_at != input.player_updated_at {
        return Err(ProfileError::Stale);
    }
    let handicap_changed = previous.handicap != input.handicap;
    if handicap_changed {
        if previous.player_active == Some(false) {
            return Err(ProfileError::Inactive);
        }
        if input.handicap.is_none() {
            return Err(ProfileError::MissingHandicap);
        }
        if input.reason.is_empty() {
            return Err(ProfileError::MissingReason);
        }
    }
    let player = if principal.player_id.is_none() && input.handicap.is_some() {
        let player = Uuid::new_v4();
        sqlx::query("INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,$2,$3)")
            .bind(player)
            .bind(&input.display_name)
            .bind(input.handicap)
            .execute(&mut *tx)
            .await?;
        Some(player)
    } else {
        principal.player_id
    };
    if let Some(player) = player {
        if previous.player_id.is_some()
            && (handicap_changed || previous.display_name != input.display_name)
        {
            sqlx::query("UPDATE players SET display_name=$2,current_handicap_index=$3 WHERE id=$1")
                .bind(player)
                .bind(&input.display_name)
                .bind(input.handicap)
                .execute(&mut *tx)
                .await?;
        }
        if handicap_changed {
            sqlx::query("INSERT INTO handicap_history(id,player_id,handicap_index,changed_by,reason) VALUES($1,$2,$3,$4,$5)")
                .bind(Uuid::new_v4()).bind(player).bind(input.handicap).bind(principal.user_id).bind(&input.reason).execute(&mut *tx).await?;
        }
    }
    if previous.display_name != input.display_name || handicap_changed {
        sqlx::query("UPDATE users SET display_name=$2,player_id=$3 WHERE id=$1")
            .bind(principal.user_id)
            .bind(&input.display_name)
            .bind(player)
            .execute(&mut *tx)
            .await?;
    }
    // All historical owners referencing this player need authoritative name reads.
    let tournaments = sqlx::query_scalar("SELECT DISTINCT tournament_id FROM tournament_players WHERE player_id=$1 ORDER BY tournament_id")
        .bind(player).fetch_all(&mut *tx).await?;
    let profile = read(&mut tx, principal.user_id).await?;
    require_live(&mut tx, session).await?;
    tx.commit().await?;
    Ok(ProfileChangeResult {
        profile,
        tournaments,
    })
}

pub async fn credential(pool: &PgPool, user: Uuid) -> Result<Option<(String, i64)>, sqlx::Error> {
    sqlx::query_as("SELECT password_hash,credential_generation FROM users WHERE id=$1 AND password_hash IS NOT NULL")
        .bind(user).fetch_optional(pool).await
}

pub enum CredentialChange {
    Username(String),
    Password(String),
}

pub async fn update_credential(
    pool: &PgPool,
    session: Uuid,
    version: i64,
    verified: &(String, i64),
    change: CredentialChange,
) -> Result<(), ProfileError> {
    let mut tx = pool.begin().await?;
    let principal = auth::lock_active_session_exclusive(&mut tx, session)
        .await?
        .ok_or(ProfileError::Unauthenticated)?;
    let current: (Option<String>, i64, i64) = sqlx::query_as(
        "SELECT password_hash,credential_generation,profile_version FROM users WHERE id=$1",
    )
    .bind(principal.user_id)
    .fetch_one(&mut *tx)
    .await?;
    require_live(&mut tx, session).await?;
    if current.0.as_ref() != Some(&verified.0) || current.1 != verified.1 {
        return Err(ProfileError::IncorrectPassword);
    }
    if current.2 != version {
        return Err(ProfileError::Stale);
    }
    match change {
        CredentialChange::Username(username) => {
            sqlx::query("UPDATE users SET username=$2 WHERE id=$1")
                .bind(principal.user_id)
                .bind(username)
                .execute(&mut *tx)
                .await?;
        }
        CredentialChange::Password(hash) => {
            sqlx::query("UPDATE users SET password_hash=$2 WHERE id=$1")
                .bind(principal.user_id)
                .bind(hash)
                .execute(&mut *tx)
                .await?;
        }
    }
    // The session's old generation is intentionally invalid after a password write;
    // its row and the user remain locked. Check only expiry at this point.
    let live: bool =
        sqlx::query_scalar("SELECT expires_at > clock_timestamp() FROM user_sessions WHERE id=$1")
            .bind(session)
            .fetch_one(&mut *tx)
            .await?;
    if !live {
        return Err(ProfileError::Unauthenticated);
    }
    tx.commit().await?;
    Ok(())
}

async fn require_live(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
) -> Result<(), ProfileError> {
    let live: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_sessions s JOIN users u ON u.id=s.user_id WHERE s.id=$1 AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp() AND s.credential_generation=u.credential_generation)")
        .bind(session).fetch_one(&mut **tx).await?;
    if !live {
        return Err(ProfileError::Unauthenticated);
    }
    Ok(())
}
