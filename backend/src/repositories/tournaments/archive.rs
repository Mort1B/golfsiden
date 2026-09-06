use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{COLUMNS, TournamentMutationError};
use crate::{
    domain::models::{Tournament, TournamentStatus},
    repositories::tournament_authorization,
};

pub struct ArchiveTournamentResult {
    pub tournament: Tournament,
    pub changed: bool,
}

pub async fn archive_authorized(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    expected_updated_at: DateTime<Utc>,
) -> Result<ArchiveTournamentResult, TournamentMutationError> {
    // No round locks: the completed plan is immutable and archive does not touch it.
    let mut transaction = pool.begin().await?;
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
    if tournament.status == TournamentStatus::Archived {
        transaction.commit().await?;
        return Ok(ArchiveTournamentResult {
            tournament,
            changed: false,
        });
    }
    if tournament.status != TournamentStatus::Completed {
        return Err(TournamentMutationError::ArchiveInvalidState);
    }
    if tournament.updated_at != expected_updated_at {
        return Err(TournamentMutationError::ArchiveStale);
    }
    sqlx::query(
        "SELECT set_config('app.tournament_archive_id', $1::text, true),
                set_config('app.tournament_archive_session_id', $2::text, true)",
    )
    .bind(tournament_id)
    .bind(session_id)
    .execute(&mut *transaction)
    .await?;
    let tournament = sqlx::query_as::<_, Tournament>(&format!(
        "UPDATE tournaments SET status = 'archived' WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(tournament_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .and_then(|error| error.constraint())
            == Some("tournament_archive_admin_required")
        {
            TournamentMutationError::Authorization(
                tournament_authorization::AuthorizationError::Unauthenticated,
            )
        } else {
            TournamentMutationError::Database(error)
        }
    })?;
    transaction.commit().await?;
    Ok(ArchiveTournamentResult {
        tournament,
        changed: true,
    })
}
