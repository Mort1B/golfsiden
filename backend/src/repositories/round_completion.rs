use sqlx::{FromRow, PgConnection, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        models::{Round, RoundStatus, ScoringFormat},
        round_completion::{
            CompletionFacts, OwnerProgressFact, RoundCompletionValidation, TransitionAction,
            TransitionBlocker, transition_blocker, validate,
        },
        scorecards::ScoreOwner,
    },
    repositories::tournament_authorization::{self, AuthorizationError},
};

const ROUND_COLUMNS: &str = "id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, status, handicap_enabled, handicap_allowance_percent, scoring_format, created_at, updated_at";

#[derive(Debug, Error)]
pub enum RoundCompletionError {
    #[error("round not found")]
    NotFound,
    #[error("round transition is blocked")]
    Blocked {
        action: TransitionAction,
        blocker: TransitionBlocker,
    },
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, FromRow)]
struct RoundFactRow {
    id: Uuid,
    status: RoundStatus,
    scoring_format: ScoringFormat,
    number_of_holes: i16,
}

#[derive(Debug, FromRow)]
struct OwnerProgressRow {
    owner_id: Uuid,
    owner_name: String,
    holes_scored: i64,
    confirmed: bool,
}

pub async fn validation(
    pool: &PgPool,
    round_id: Uuid,
) -> Result<Option<RoundCompletionValidation>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *transaction)
        .await?;
    let facts = load_facts(&mut transaction, round_id, false).await?;
    let validation = facts.map(validate);
    transaction.commit().await?;
    Ok(validation)
}

pub async fn validation_for_member(
    pool: &PgPool,
    user_id: Uuid,
    round_id: Uuid,
) -> Result<RoundCompletionValidation, AuthorizationError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    tournament_authorization::require_round_member_read(&mut transaction, user_id, round_id)
        .await?;
    let validation = load_facts(&mut transaction, round_id, false)
        .await?
        .map(validate)
        .ok_or(AuthorizationError::NotFound)?;
    transaction.commit().await?;
    Ok(validation)
}

pub async fn complete(pool: &PgPool, round_id: Uuid) -> Result<Round, RoundCompletionError> {
    transition(pool, round_id, TransitionAction::Complete).await
}

pub async fn lock(pool: &PgPool, round_id: Uuid) -> Result<Round, RoundCompletionError> {
    transition(pool, round_id, TransitionAction::Lock).await
}

pub async fn complete_authorized(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
) -> Result<Round, RoundCompletionError> {
    transition_authorized(pool, session_id, round_id, TransitionAction::Complete).await
}

pub async fn lock_authorized(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
) -> Result<Round, RoundCompletionError> {
    transition_authorized(pool, session_id, round_id, TransitionAction::Lock).await
}

async fn transition(
    pool: &PgPool,
    round_id: Uuid,
    action: TransitionAction,
) -> Result<Round, RoundCompletionError> {
    let mut transaction = pool.begin().await?;
    let round = transition_in_transaction(&mut transaction, round_id, action).await?;
    transaction.commit().await?;
    Ok(round)
}

async fn transition_authorized(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    action: TransitionAction,
) -> Result<Round, RoundCompletionError> {
    let mut transaction = pool.begin().await?;
    tournament_authorization::require_round_admin(&mut transaction, session_id, round_id).await?;
    let round = transition_in_transaction(&mut transaction, round_id, action).await?;
    transaction.commit().await?;
    Ok(round)
}

async fn transition_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
    action: TransitionAction,
) -> Result<Round, RoundCompletionError> {
    let facts = load_facts(transaction, round_id, true)
        .await?
        .ok_or(RoundCompletionError::NotFound)?;
    let validation = validate(facts);
    if let Some(blocker) = transition_blocker(&validation, action) {
        return Err(RoundCompletionError::Blocked { action, blocker });
    }

    let (setting, source_status, target_status) = match action {
        TransitionAction::Complete => (
            "app.round_completion_id",
            RoundStatus::Open,
            RoundStatus::Completed,
        ),
        TransitionAction::Lock => (
            "app.round_lock_id",
            RoundStatus::Completed,
            RoundStatus::Locked,
        ),
    };
    sqlx::query("SELECT set_config($1, $2::text, true)")
        .bind(setting)
        .bind(round_id)
        .execute(&mut **transaction)
        .await?;
    let update = sqlx::query("UPDATE rounds SET status = $2 WHERE id = $1 AND status = $3")
        .bind(round_id)
        .bind(target_status)
        .bind(source_status)
        .execute(&mut **transaction)
        .await?;
    if update.rows_affected() != 1 {
        return Err(RoundCompletionError::Blocked {
            action,
            blocker: TransitionBlocker::InvalidSourceState,
        });
    }
    let round =
        sqlx::query_as::<_, Round>(&format!("SELECT {ROUND_COLUMNS} FROM rounds WHERE id = $1"))
            .bind(round_id)
            .fetch_one(&mut **transaction)
            .await?;
    Ok(round)
}

async fn load_facts(
    connection: &mut PgConnection,
    round_id: Uuid,
    lock: bool,
) -> Result<Option<CompletionFacts>, sqlx::Error> {
    let sql = if lock {
        "SELECT id, status, scoring_format, number_of_holes FROM rounds WHERE id = $1 FOR UPDATE"
    } else {
        "SELECT id, status, scoring_format, number_of_holes FROM rounds WHERE id = $1"
    };
    let Some(round) = sqlx::query_as::<_, RoundFactRow>(sql)
        .bind(round_id)
        .fetch_optional(&mut *connection)
        .await?
    else {
        return Ok(None);
    };
    let owners = match round.scoring_format {
        ScoringFormat::IndividualStrokePlay => load_individual_owners(connection, round.id).await?,
        ScoringFormat::TeamScramble => load_team_owners(connection, round.id).await?,
    };
    Ok(Some(CompletionFacts {
        round_id: round.id,
        status: round.status,
        required_holes: round.number_of_holes,
        owners,
    }))
}

async fn load_individual_owners(
    connection: &mut PgConnection,
    round_id: Uuid,
) -> Result<Vec<OwnerProgressFact>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OwnerProgressRow>(
        "SELECT rhs.player_id AS owner_id, p.display_name AS owner_name, count(s.id) AS holes_scored, EXISTS(SELECT 1 FROM scorecard_confirmations sc WHERE sc.round_id = rhs.round_id AND sc.player_id = rhs.player_id) AS confirmed FROM round_handicap_snapshots rhs JOIN players p ON p.id = rhs.player_id LEFT JOIN scores s ON s.round_id = rhs.round_id AND s.player_id = rhs.player_id WHERE rhs.round_id = $1 GROUP BY rhs.round_id, rhs.player_id, p.display_name ORDER BY p.display_name, rhs.player_id",
    )
    .bind(round_id)
    .fetch_all(connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| OwnerProgressFact {
            owner: ScoreOwner::Player { id: row.owner_id },
            owner_name: row.owner_name,
            holes_scored: row.holes_scored,
            confirmed: row.confirmed,
        })
        .collect())
}

async fn load_team_owners(
    connection: &mut PgConnection,
    round_id: Uuid,
) -> Result<Vec<OwnerProgressFact>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OwnerProgressRow>(
        "SELECT t.id AS owner_id, t.name AS owner_name, count(s.id) AS holes_scored, EXISTS(SELECT 1 FROM scorecard_confirmations sc WHERE sc.round_id = t.round_id AND sc.team_id = t.id) AS confirmed FROM teams t LEFT JOIN scores s ON s.round_id = t.round_id AND s.team_id = t.id WHERE t.round_id = $1 GROUP BY t.round_id, t.id, t.name ORDER BY t.name, t.id",
    )
    .bind(round_id)
    .fetch_all(connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| OwnerProgressFact {
            owner: ScoreOwner::Team { id: row.owner_id },
            owner_name: row.owner_name,
            holes_scored: row.holes_scored,
            confirmed: row.confirmed,
        })
        .collect())
}
