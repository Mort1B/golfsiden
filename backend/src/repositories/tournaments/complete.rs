use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{COLUMNS, TournamentMutationError};
use crate::{
    domain::models::{Tournament, TournamentStatus},
    repositories::tournament_authorization,
};

pub struct CompleteTournamentResult {
    pub tournament: Tournament,
    pub changed: bool,
}

pub async fn complete_authorized(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    expected_updated_at: DateTime<Utc>,
) -> Result<CompleteTournamentResult, TournamentMutationError> {
    // Preflight prevents unrelated callers from waiting on private round locks.
    let mut preflight = pool.begin().await?;
    tournament_authorization::require_tournament_admin(&mut preflight, session_id, tournament_id)
        .await?;
    preflight.rollback().await?;

    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT id FROM rounds WHERE tournament_id = $1 ORDER BY id FOR UPDATE")
        .bind(tournament_id)
        .fetch_all(&mut *transaction)
        .await?;
    tournament_authorization::require_tournament_admin(&mut transaction, session_id, tournament_id)
        .await?;
    let tournament = sqlx::query_as::<_, Tournament>(&format!(
        "SELECT {COLUMNS} FROM tournaments WHERE id = $1 FOR UPDATE"
    ))
    .bind(tournament_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(TournamentMutationError::NotFound)?;
    // Recheck wall-clock expiry after all lock waits, including idempotent retries.
    let active = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM user_sessions WHERE id = $1
         AND revoked_at IS NULL AND expires_at > clock_timestamp())",
    )
    .bind(session_id)
    .fetch_one(&mut *transaction)
    .await?;
    if !active {
        return Err(tournament_authorization::AuthorizationError::Unauthenticated.into());
    }
    if tournament.status == TournamentStatus::Completed {
        transaction.commit().await?;
        return Ok(CompleteTournamentResult {
            tournament,
            changed: false,
        });
    }
    if tournament.status != TournamentStatus::Active {
        return Err(TournamentMutationError::CompletionInvalidState);
    }
    if tournament.updated_at != expected_updated_at {
        return Err(TournamentMutationError::CompletionStale);
    }
    // Re-read after locking the parent: round inserts before that lock must not
    // escape the exact-plan check. Join/round-plan guards hold this parent lock.
    let ready = sqlx::query_scalar::<_, bool>(
        "SELECT count(*) = $2 AND count(*) FILTER
         (WHERE status = 'locked' AND round_number BETWEEN 1 AND $2) = $2
         FROM rounds WHERE tournament_id = $1",
    )
    .bind(tournament_id)
    .bind(i64::from(tournament.number_of_rounds))
    .fetch_one(&mut *transaction)
    .await?;
    if !ready {
        return Err(TournamentMutationError::CompletionNotReady);
    }
    sqlx::query(
        "SELECT set_config('app.tournament_completion_id', $1::text, true),
                set_config('app.tournament_completion_session_id', $2::text, true)",
    )
    .bind(tournament_id)
    .bind(session_id)
    .execute(&mut *transaction)
    .await?;
    let tournament = sqlx::query_as::<_, Tournament>(&format!(
        "UPDATE tournaments SET status = 'completed' WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(tournament_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .and_then(|error| error.constraint())
            == Some("tournament_completion_admin_required")
        {
            TournamentMutationError::Authorization(
                tournament_authorization::AuthorizationError::Unauthenticated,
            )
        } else {
            TournamentMutationError::Database(error)
        }
    })?;
    transaction.commit().await?;
    Ok(CompleteTournamentResult {
        tournament,
        changed: true,
    })
}
