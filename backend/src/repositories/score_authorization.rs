use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    auth::SessionPrincipal,
    domain::{
        models::{ScoringFormat, TournamentRole},
        scorecards::ScoreOwner,
    },
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
    let context = sqlx::query_as::<_, (Uuid, ScoringFormat)>(
        "SELECT tournament_id, scoring_format FROM rounds WHERE id = $1",
    )
    .bind(round_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(ScoreAuthorizationError::NotFound)?;
    let role = membership_role(&mut connection, context.0, principal.user_id).await?;
    list_for_principal(&mut connection, principal, role, round_id, context.1).await
}

pub async fn authorize_mutation(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    round_id: Uuid,
    format: ScoringFormat,
    owner: ScoreOwner,
) -> Result<Uuid, ScoreAuthorizationError> {
    let tournament_id =
        sqlx::query_scalar::<_, Uuid>("SELECT tournament_id FROM rounds WHERE id = $1")
            .bind(round_id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or(ScoreAuthorizationError::NotFound)?;
    let principal = auth::lock_active_session(transaction, session_id)
        .await?
        .ok_or(ScoreAuthorizationError::Unauthenticated)?;
    let role = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2 FOR SHARE",
    )
    .bind(tournament_id)
    .bind(principal.user_id)
    .fetch_optional(&mut **transaction)
    .await?;
    if owner_allowed(transaction, &principal, role, round_id, format, owner).await? {
        Ok(principal.user_id)
    } else {
        Err(ScoreAuthorizationError::Forbidden)
    }
}

async fn list_for_principal(
    connection: &mut PgConnection,
    principal: &SessionPrincipal,
    role: Option<TournamentRole>,
    round_id: Uuid,
    format: ScoringFormat,
) -> Result<Vec<ScoreOwner>, ScoreAuthorizationError> {
    match format {
        ScoringFormat::IndividualStrokePlay => {
            let ids = if matches!(role, Some(TournamentRole::Admin | TournamentRole::Scorer)) {
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
            } else if role == Some(TournamentRole::Player) {
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
            let ids = if matches!(role, Some(TournamentRole::Admin | TournamentRole::Scorer)) {
                sqlx::query_scalar::<_, Uuid>(
                    "SELECT id FROM teams WHERE round_id = $1 ORDER BY lower(name), id",
                )
                .bind(round_id)
                .fetch_all(connection)
                .await?
            } else if role == Some(TournamentRole::Player) {
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
    role: Option<TournamentRole>,
    round_id: Uuid,
    format: ScoringFormat,
    owner: ScoreOwner,
) -> Result<bool, sqlx::Error> {
    if matches!(role, Some(TournamentRole::Admin | TournamentRole::Scorer)) {
        return Ok(true);
    }
    if role != Some(TournamentRole::Player) {
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

async fn membership_role(
    connection: &mut PgConnection,
    tournament_id: Uuid,
    user_id: Uuid,
) -> Result<Option<TournamentRole>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2",
    )
    .bind(tournament_id)
    .bind(user_id)
    .fetch_optional(connection)
    .await
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
