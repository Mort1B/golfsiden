use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        models::{Round, RoundStatus, ScoringFormat, TournamentRole},
        round_completion::{
            CompletionFacts, OwnerProgressFact, RoundCompletionReadProjection,
            RoundCompletionValidation, TransitionAction, TransitionBlocker, read_projection,
            transition_blocker, validate,
        },
        round_formats::{RoundFormatPolicy, ScoreOwnerKind},
        score_visibility::{VisibilityFacts, visibility},
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

#[derive(Debug, FromRow)]
struct ReadVisibilityRow {
    tournament_id: Uuid,
    round_number: i16,
    status: RoundStatus,
    number_of_holes: i16,
    final_scores_hidden_until: Option<DateTime<Utc>>,
    tournament_round_count: i16,
}

#[derive(Debug, FromRow)]
struct VisibleScoreRow {
    player_id: Option<Uuid>,
    team_id: Option<Uuid>,
    holes_scored: i64,
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
) -> Result<RoundCompletionReadProjection, AuthorizationError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    let context = sqlx::query_as::<_, ReadVisibilityRow>(
        "SELECT r.tournament_id, r.round_number, r.status, r.number_of_holes,
                r.final_scores_hidden_until, t.number_of_rounds AS tournament_round_count
         FROM rounds r JOIN tournaments t ON t.id = r.tournament_id WHERE r.id = $1",
    )
    .bind(round_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AuthorizationError::NotFound)?;
    tournament_authorization::require_tournament_member_read(
        &mut transaction,
        user_id,
        context.tournament_id,
    )
    .await?;
    let role = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships WHERE tournament_id = $1 AND user_id = $2 FOR SHARE",
    )
    .bind(context.tournament_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AuthorizationError::Forbidden)?;
    let validation = load_facts(&mut transaction, round_id, false)
        .await?
        .map(validate)
        .ok_or(AuthorizationError::NotFound)?;
    let observed_at = sqlx::query_scalar("SELECT transaction_timestamp()")
        .fetch_one(&mut *transaction)
        .await?;
    let metadata = completion_visibility(&context, role, observed_at);
    let visible_holes = load_visible_holes(&mut transaction, round_id).await?;
    let projection = read_projection(validation, metadata, &visible_holes);
    transaction.commit().await?;
    Ok(projection)
}

fn completion_visibility(
    context: &ReadVisibilityRow,
    role: TournamentRole,
    observed_at: DateTime<Utc>,
) -> crate::domain::score_visibility::VisibilityMetadata {
    visibility(VisibilityFacts {
        role,
        is_final_round: context.round_number == context.tournament_round_count,
        status: context.status,
        number_of_holes: context.number_of_holes,
        hidden_until: context.final_scores_hidden_until,
        observed_at,
    })
}

async fn load_visible_holes(
    connection: &mut PgConnection,
    round_id: Uuid,
) -> Result<HashMap<ScoreOwner, i64>, sqlx::Error> {
    let rows = sqlx::query_as::<_, VisibleScoreRow>(
        "SELECT s.player_id, s.team_id, count(*) AS holes_scored
         FROM scores s
         JOIN rounds r ON r.id = s.round_id
         JOIN holes h ON h.id = s.hole_id AND h.tee_id = r.tee_id
         WHERE s.round_id = $1 AND h.hole_number <= 9
         GROUP BY s.player_id, s.team_id",
    )
    .bind(round_id)
    .fetch_all(connection)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| match (row.player_id, row.team_id) {
            (Some(id), None) => Some((ScoreOwner::Player { id }, row.holes_scored)),
            (None, Some(id)) => Some((ScoreOwner::Team { id }, row.holes_scored)),
            _ => None,
        })
        .collect())
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
    let owners = match RoundFormatPolicy::for_format(round.scoring_format).owner_kind() {
        ScoreOwnerKind::Player => load_individual_owners(connection, round.id).await?,
        ScoreOwnerKind::Team => load_team_owners(connection, round.id).await?,
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
        "SELECT t.id AS owner_id, t.name AS owner_name, count(s.id) AS holes_scored,
                EXISTS(SELECT 1 FROM scorecard_confirmations sc WHERE sc.round_id = t.round_id AND sc.team_id = t.id)
                AND (r.scoring_format <> 'two_player_foursomes'
                     OR EXISTS(SELECT 1 FROM round_team_handicap_snapshots rths WHERE rths.round_id = t.round_id AND rths.team_id = t.id)) AS confirmed
         FROM teams t JOIN rounds r ON r.id = t.round_id
         LEFT JOIN scores s ON s.round_id = t.round_id AND s.team_id = t.id
         WHERE t.round_id = $1
         GROUP BY t.round_id, t.id, t.name, r.scoring_format ORDER BY t.name, t.id",
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

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::domain::score_visibility::VisibilityMode;

    #[test]
    fn completion_read_wiring_reveals_at_exact_database_deadline() {
        let observed_at = Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap();
        let mut context = ReadVisibilityRow {
            tournament_id: Uuid::from_u128(1),
            round_number: 4,
            status: RoundStatus::Completed,
            number_of_holes: 18,
            final_scores_hidden_until: Some(observed_at),
            tournament_round_count: 4,
        };
        assert_eq!(
            completion_visibility(&context, TournamentRole::Player, observed_at).mode,
            VisibilityMode::Full
        );
        context.final_scores_hidden_until = Some(observed_at + chrono::Duration::nanoseconds(1));
        assert_eq!(
            completion_visibility(&context, TournamentRole::Player, observed_at).mode,
            VisibilityMode::FrontNine
        );
    }
}
