use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::models::{Round, ScoringFormat};

const COLUMNS: &str = "id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format, created_at, updated_at";

pub async fn list(pool: &PgPool, tournament_id: Uuid) -> Result<Vec<Round>, sqlx::Error> {
    sqlx::query_as::<_, Round>(&format!(
        "SELECT {COLUMNS} FROM rounds WHERE tournament_id = $1 ORDER BY round_number"
    ))
    .bind(tournament_id)
    .fetch_all(pool)
    .await
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<Round>, sqlx::Error> {
    sqlx::query_as::<_, Round>(&format!("SELECT {COLUMNS} FROM rounds WHERE id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    pool: &PgPool,
    tournament_id: Uuid,
    round_number: i16,
    name: &str,
    round_date: NaiveDate,
    course_name: &str,
    tee_name: &str,
    number_of_holes: i16,
    handicap_enabled: bool,
    handicap_allowance_percent: i16,
    scoring_format: ScoringFormat,
) -> Result<Round, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_name, tee_name, number_of_holes, handicap_enabled, handicap_allowance_percent, scoring_format) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
        .bind(id).bind(tournament_id).bind(round_number).bind(name.trim()).bind(round_date)
        .bind(course_name.trim()).bind(tee_name.trim()).bind(number_of_holes).bind(handicap_enabled)
        .bind(handicap_allowance_percent).bind(scoring_format).execute(pool).await?;
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}
