use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::models::{HandicapHistoryEntry, Player};

const PLAYER_COLUMNS: &str = "id, display_name, current_handicap_index::float8 AS current_handicap_index, email, profile_image_ref, active, created_at, updated_at";

pub async fn list(pool: &PgPool) -> Result<Vec<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>(&format!(
        "SELECT {PLAYER_COLUMNS} FROM players ORDER BY active DESC, display_name"
    ))
    .fetch_all(pool)
    .await
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>(&format!(
        "SELECT {PLAYER_COLUMNS} FROM players WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn create(
    pool: &PgPool,
    display_name: &str,
    handicap: f64,
    email: Option<&str>,
    profile_image_ref: Option<&str>,
) -> Result<Player, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO players (id, display_name, current_handicap_index, email, profile_image_ref) VALUES ($1, $2, $3, $4, $5)")
        .bind(id).bind(display_name.trim()).bind(handicap).bind(email).bind(profile_image_ref)
        .execute(&mut *tx).await?;
    insert_handicap_history(&mut tx, id, handicap, None, Some("initial handicap")).await?;
    tx.commit().await?;
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    display_name: Option<&str>,
    email: Option<&str>,
    profile_image_ref: Option<&str>,
    active: Option<bool>,
) -> Result<Option<Player>, sqlx::Error> {
    sqlx::query("UPDATE players SET display_name = COALESCE($2, display_name), email = COALESCE($3, email), profile_image_ref = COALESCE($4, profile_image_ref), active = COALESCE($5, active) WHERE id = $1")
        .bind(id).bind(display_name.map(str::trim)).bind(email).bind(profile_image_ref).bind(active)
        .execute(pool).await?;
    get(pool, id).await
}

pub async fn deactivate(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("UPDATE players SET active = FALSE WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected()
            == 1,
    )
}

pub async fn change_handicap(
    pool: &PgPool,
    id: Uuid,
    handicap: f64,
    changed_by: Option<Uuid>,
    reason: Option<&str>,
) -> Result<Option<HandicapHistoryEntry>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query("UPDATE players SET current_handicap_index = $2 WHERE id = $1")
        .bind(id)
        .bind(handicap)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    let entry = insert_handicap_history(&mut tx, id, handicap, changed_by, reason).await?;
    tx.commit().await?;
    Ok(Some(entry))
}

pub async fn handicap_history(
    pool: &PgPool,
    id: Uuid,
) -> Result<Vec<HandicapHistoryEntry>, sqlx::Error> {
    sqlx::query_as::<_, HandicapHistoryEntry>("SELECT id, player_id, handicap_index::float8 AS handicap_index, effective_from, changed_by, reason, created_at FROM handicap_history WHERE player_id = $1 ORDER BY effective_from DESC")
        .bind(id).fetch_all(pool).await
}

async fn insert_handicap_history(
    tx: &mut Transaction<'_, Postgres>,
    player_id: Uuid,
    handicap: f64,
    changed_by: Option<Uuid>,
    reason: Option<&str>,
) -> Result<HandicapHistoryEntry, sqlx::Error> {
    let entry = HandicapHistoryEntry {
        id: Uuid::new_v4(),
        player_id,
        handicap_index: handicap,
        effective_from: Utc::now(),
        changed_by,
        reason: reason.map(str::to_owned),
        created_at: Utc::now(),
    };
    sqlx::query("INSERT INTO handicap_history (id, player_id, handicap_index, effective_from, changed_by, reason, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(entry.id).bind(entry.player_id).bind(entry.handicap_index).bind(entry.effective_from)
        .bind(entry.changed_by).bind(&entry.reason).bind(entry.created_at).execute(&mut **tx).await?;
    Ok(entry)
}
