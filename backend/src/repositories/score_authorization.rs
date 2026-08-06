use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    auth::{SessionPrincipal, UserRole},
    domain::{models::ScoringFormat, scorecards::ScoreOwner},
    repositories::auth,
};

#[derive(Debug, Error)]
pub enum ScoreAuthorizationError {
    #[error("resource not found")]
    NotFound,
    #[error("session is not authenticated")]
    Unauthenticated,
    #[error("session cannot write this scorecard")]
    Forbidden,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn writable_owners(
    pool: &PgPool,
    principal: &SessionPrincipal,
    round_id: Uuid,
) -> Result<Vec<ScoreOwner>, ScoreAuthorizationError> {
    let mut connection = pool.acquire().await?;
    let format =
        sqlx::query_scalar::<_, ScoringFormat>("SELECT scoring_format FROM rounds WHERE id = $1")
            .bind(round_id)
            .fetch_optional(&mut *connection)
            .await?
            .ok_or(ScoreAuthorizationError::NotFound)?;
    list_for_principal(&mut connection, principal, round_id, format).await
}

pub async fn authorize_mutation(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    round_id: Uuid,
    format: ScoringFormat,
    owner: ScoreOwner,
) -> Result<Uuid, ScoreAuthorizationError> {
    let principal = auth::lock_active_session(transaction, session_id)
        .await?
        .ok_or(ScoreAuthorizationError::Unauthenticated)?;
    if owner_allowed(transaction, &principal, round_id, format, owner).await? {
        Ok(principal.user_id)
    } else {
        Err(ScoreAuthorizationError::Forbidden)
    }
}

async fn list_for_principal(
    connection: &mut PgConnection,
    principal: &SessionPrincipal,
    round_id: Uuid,
    format: ScoringFormat,
) -> Result<Vec<ScoreOwner>, ScoreAuthorizationError> {
    if matches!(principal.role, UserRole::Viewer) {
        return Ok(Vec::new());
    }
    match format {
        ScoringFormat::IndividualStrokePlay => {
            let ids = if auth::can_score_all(principal.role) {
                sqlx::query_scalar::<_, Uuid>(
                    "SELECT rhs.player_id
                     FROM round_handicap_snapshots rhs
                     JOIN players p ON p.id = rhs.player_id
                     WHERE rhs.round_id = $1
                     ORDER BY lower(p.display_name), p.id",
                )
                .bind(round_id)
                .fetch_all(connection)
                .await?
            } else if principal.role == UserRole::Player {
                let Some(player_id) = principal.player_id else {
                    return Ok(Vec::new());
                };
                sqlx::query_scalar::<_, Uuid>(
                    "SELECT player_id FROM round_handicap_snapshots
                     WHERE round_id = $1 AND player_id = $2",
                )
                .bind(round_id)
                .bind(player_id)
                .fetch_optional(connection)
                .await?
                .into_iter()
                .collect()
            } else {
                Vec::new()
            };
            Ok(ids
                .into_iter()
                .map(|id| ScoreOwner::Player { id })
                .collect())
        }
        ScoringFormat::TeamScramble => {
            let ids = if auth::can_score_all(principal.role) {
                sqlx::query_scalar::<_, Uuid>(
                    "SELECT id FROM teams WHERE round_id = $1 ORDER BY lower(name), id",
                )
                .bind(round_id)
                .fetch_all(connection)
                .await?
            } else if principal.role == UserRole::Player {
                let Some(player_id) = principal.player_id else {
                    return Ok(Vec::new());
                };
                direct_team_ids(connection, round_id, player_id).await?
            } else {
                Vec::new()
            };
            Ok(ids.into_iter().map(|id| ScoreOwner::Team { id }).collect())
        }
    }
}

async fn owner_allowed(
    connection: &mut PgConnection,
    principal: &SessionPrincipal,
    round_id: Uuid,
    format: ScoringFormat,
    owner: ScoreOwner,
) -> Result<bool, sqlx::Error> {
    if auth::can_score_all(principal.role) {
        return Ok(true);
    }
    if principal.role != UserRole::Player {
        return Ok(false);
    }
    let Some(player_id) = principal.player_id else {
        return Ok(false);
    };
    match (format, owner) {
        (ScoringFormat::IndividualStrokePlay, ScoreOwner::Player { id }) => Ok(id == player_id),
        (ScoringFormat::TeamScramble, ScoreOwner::Team { id }) => {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                    SELECT 1 FROM team_memberships
                    WHERE round_id = $1 AND team_id = $2 AND player_id = $3
                 )",
            )
            .bind(round_id)
            .bind(id)
            .bind(player_id)
            .fetch_one(connection)
            .await
        }
        _ => Ok(false),
    }
}

async fn direct_team_ids(
    connection: &mut PgConnection,
    round_id: Uuid,
    player_id: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    // Future flight membership expands this ordered set; mutation checks call the same policy.
    sqlx::query_scalar(
        "SELECT t.id
         FROM team_memberships tm
         JOIN teams t ON t.id = tm.team_id AND t.round_id = tm.round_id
         WHERE tm.round_id = $1 AND tm.player_id = $2
         ORDER BY lower(t.name), t.id",
    )
    .bind(round_id)
    .bind(player_id)
    .fetch_all(connection)
    .await
}
