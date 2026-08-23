use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        course_revisions::ValidatedCourseRevision,
        models::{Round, RoundStatus},
    },
    repositories::{
        course_revisions::{self, CourseRevisionRepositoryError},
        tournament_authorization::{self, AuthorizationError},
    },
};

const ROUND_COLUMNS: &str = "id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format, created_at, updated_at";

#[derive(Debug, Error)]
pub enum RoundConfigurationError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("round is not draft")]
    NotDraft,
    #[error("round configuration has changed")]
    Stale,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("course revision could not be persisted")]
    CourseRevision(#[from] CourseRevisionRepositoryError),
}

#[derive(sqlx::FromRow)]
struct RoundPreflight {
    tournament_id: Uuid,
    status: RoundStatus,
    updated_at: DateTime<Utc>,
}

pub struct ConfigurationPreflight {
    pub updated_at: DateTime<Utc>,
}

pub async fn preflight(
    pool: &PgPool,
    user_id: Uuid,
    round_id: Uuid,
) -> Result<ConfigurationPreflight, RoundConfigurationError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    let round = load_preflight(&mut transaction, round_id).await?;
    tournament_authorization::require_tournament_admin_read_in_transaction(
        &mut transaction,
        user_id,
        round.tournament_id,
    )
    .await?;
    if round.status != RoundStatus::Draft {
        return Err(RoundConfigurationError::NotDraft);
    }
    transaction.commit().await?;
    Ok(ConfigurationPreflight {
        updated_at: round.updated_at,
    })
}

pub async fn configure(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    expected_updated_at: DateTime<Utc>,
    revision: &ValidatedCourseRevision,
) -> Result<Round, RoundConfigurationError> {
    let mut transaction = pool.begin().await?;
    tournament_authorization::require_round_admin(&mut transaction, session_id, round_id).await?;
    let round = load_preflight(&mut transaction, round_id).await?;
    check_round(&round, expected_updated_at)?;

    let inserted = course_revisions::insert_in_transaction(&mut transaction, revision).await?;
    let updated = sqlx::query_as::<_, Round>(&format!(
        "UPDATE rounds
         SET course_id = $2, course_name = $3, tee_id = $4, tee_name = $5,
             number_of_holes = $6
         WHERE id = $1
         RETURNING {ROUND_COLUMNS}"
    ))
    .bind(round_id)
    .bind(inserted.course_id)
    .bind(&inserted.course_name)
    .bind(inserted.tee.tee_id)
    .bind(&inserted.tee.name)
    .bind(
        i16::try_from(inserted.tee.holes.len())
            .map_err(|_| CourseRevisionRepositoryError::InvalidStoredRevision)?,
    )
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(updated)
}

async fn load_preflight(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
) -> Result<RoundPreflight, RoundConfigurationError> {
    sqlx::query_as("SELECT tournament_id, status, updated_at FROM rounds WHERE id = $1")
        .bind(round_id)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(AuthorizationError::NotFound.into())
}

fn check_round(
    round: &RoundPreflight,
    expected_updated_at: DateTime<Utc>,
) -> Result<(), RoundConfigurationError> {
    if round.status != RoundStatus::Draft {
        return Err(RoundConfigurationError::NotDraft);
    }
    if round.updated_at != expected_updated_at {
        return Err(RoundConfigurationError::Stale);
    }
    Ok(())
}
