use chrono::NaiveDate;
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::models::{Round, ScoringFormat},
    repositories::tournament_authorization::{self, AuthorizationError},
};

const COLUMNS: &str = "id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format, created_at, updated_at";

#[derive(Debug, Error)]
pub enum RoundMutationError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn list(pool: &PgPool, tournament_id: Uuid) -> Result<Vec<Round>, sqlx::Error> {
    sqlx::query_as::<_, Round>(&format!(
        "SELECT {COLUMNS} FROM rounds WHERE tournament_id = $1 ORDER BY round_number, id"
    ))
    .bind(tournament_id)
    .fetch_all(pool)
    .await
}

pub async fn list_for_member(
    pool: &PgPool,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<Vec<Round>, AuthorizationError> {
    let mut transaction = read_transaction(pool).await?;
    tournament_authorization::require_tournament_member_read(
        &mut transaction,
        user_id,
        tournament_id,
    )
    .await?;
    let rounds = sqlx::query_as::<_, Round>(&format!(
        "SELECT {COLUMNS} FROM rounds
         WHERE tournament_id = $1 ORDER BY round_number, id"
    ))
    .bind(tournament_id)
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(rounds)
}

pub async fn get_for_member(
    pool: &PgPool,
    user_id: Uuid,
    round_id: Uuid,
) -> Result<Round, AuthorizationError> {
    let mut transaction = read_transaction(pool).await?;
    tournament_authorization::require_round_member_read(&mut transaction, user_id, round_id)
        .await?;
    let round = sqlx::query_as::<_, Round>(&format!("SELECT {COLUMNS} FROM rounds WHERE id = $1"))
        .bind(round_id)
        .fetch_one(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(round)
}

async fn read_transaction(pool: &PgPool) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    Ok(transaction)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<Round>, sqlx::Error> {
    sqlx::query_as::<_, Round>(&format!("SELECT {COLUMNS} FROM rounds WHERE id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub struct CreateRoundParams<'a> {
    pub tournament_id: Uuid,
    pub round_number: i16,
    pub name: &'a str,
    pub round_date: NaiveDate,
    pub course_id: Option<Uuid>,
    pub course_name: &'a str,
    pub tee_id: Option<Uuid>,
    pub tee_name: &'a str,
    pub number_of_holes: i16,
    pub handicap_enabled: bool,
    pub handicap_allowance_percent: i16,
    pub scoring_format: ScoringFormat,
}

pub async fn create(pool: &PgPool, input: CreateRoundParams<'_>) -> Result<Round, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, handicap_enabled, handicap_allowance_percent, scoring_format) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)")
        .bind(id)
        .bind(input.tournament_id)
        .bind(input.round_number)
        .bind(input.name.trim())
        .bind(input.round_date)
        .bind(input.course_id)
        .bind(input.course_name.trim())
        .bind(input.tee_id)
        .bind(input.tee_name.trim())
        .bind(input.number_of_holes)
        .bind(input.handicap_enabled)
        .bind(input.handicap_allowance_percent)
        .bind(input.scoring_format)
        .execute(pool)
        .await?;
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn create_authorized(
    pool: &PgPool,
    session_id: Uuid,
    input: CreateRoundParams<'_>,
) -> Result<Round, RoundMutationError> {
    let mut transaction = pool.begin().await?;
    tournament_authorization::require_tournament_admin(
        &mut transaction,
        session_id,
        input.tournament_id,
    )
    .await?;
    let id = Uuid::new_v4();
    let round = sqlx::query_as::<_, Round>(&format!(
        "INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, handicap_enabled, handicap_allowance_percent, scoring_format)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(input.tournament_id)
    .bind(input.round_number)
    .bind(input.name.trim())
    .bind(input.round_date)
    .bind(input.course_id)
    .bind(input.course_name.trim())
    .bind(input.tee_id)
    .bind(input.tee_name.trim())
    .bind(input.number_of_holes)
    .bind(input.handicap_enabled)
    .bind(input.handicap_allowance_percent)
    .bind(input.scoring_format)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(round)
}

pub async fn course_tee_matches(
    pool: &PgPool,
    course_id: Uuid,
    tee_id: Uuid,
    course_name: &str,
    tee_name: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tees t JOIN courses c ON c.id = t.course_id WHERE t.id = $1 AND c.id = $2 AND c.name = $3 AND t.name = $4)",
    )
    .bind(tee_id)
    .bind(course_id)
    .bind(course_name.trim())
    .bind(tee_name.trim())
    .fetch_one(pool)
    .await
}
