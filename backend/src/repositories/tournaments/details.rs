use chrono::{DateTime, Utc};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use super::COLUMNS;
use crate::{
    domain::{
        models::{Tournament, TournamentStatus},
        tournament_details::TournamentDetails,
    },
    repositories::{
        auth,
        tournament_authorization::{self, AuthorizationError},
    },
};

#[derive(Debug, Error)]
pub enum DetailsError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("tournament details are locked")]
    Locked,
    #[error("tournament details changed")]
    Stale,
    #[error("round dates are outside the requested range")]
    RoundDates,
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub struct DetailsResult {
    pub tournament: Tournament,
    pub changed: bool,
}

pub async fn update(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    expected_updated_at: DateTime<Utc>,
    input: TournamentDetails,
) -> Result<DetailsResult, DetailsError> {
    let input = input.normalize().map_err(DetailsError::Invalid)?;
    // Reject unauthorized callers before acquiring the round mutation locks.
    let mut preflight = pool.begin().await?;
    tournament_authorization::require_tournament_admin(&mut preflight, session_id, tournament_id)
        .await?;
    preflight.rollback().await?;

    let mut tx = pool.begin().await?;
    // Match tournament start/configuration: rounds, then authority, then parent.
    sqlx::query("SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE")
        .bind(tournament_id)
        .fetch_all(&mut *tx)
        .await?;
    tournament_authorization::require_tournament_admin(&mut tx, session_id, tournament_id).await?;
    let tournament = sqlx::query_as::<_, Tournament>(&format!(
        "SELECT {COLUMNS} FROM tournaments WHERE id=$1 FOR UPDATE"
    ))
    .bind(tournament_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AuthorizationError::NotFound)?;
    if auth::lock_active_session(&mut tx, session_id)
        .await?
        .is_none()
    {
        return Err(AuthorizationError::Unauthenticated.into());
    }
    if tournament.status != TournamentStatus::Draft {
        return Err(DetailsError::Locked);
    }
    if tournament.updated_at != expected_updated_at {
        return Err(DetailsError::Stale);
    }
    // Read again after the parent lock: concurrent round inserts may have committed
    // while this transaction was waiting. The parent lock now excludes new ones.
    let outside: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rounds WHERE tournament_id=$1 AND (round_date < $2 OR round_date > $3))")
        .bind(tournament_id).bind(input.start_date).bind(input.end_date).fetch_one(&mut *tx).await?;
    if outside {
        return Err(DetailsError::RoundDates);
    }
    let changed = tournament.name != input.name
        || tournament.description != input.description
        || tournament.start_date != input.start_date
        || tournament.end_date != input.end_date;
    let tournament = if changed {
        sqlx::query("SELECT set_config('app.tournament_details_id', $1::text, true), set_config('app.tournament_details_session_id', $2::text, true)")
            .bind(tournament_id).bind(session_id).execute(&mut *tx).await?;
        sqlx::query_as::<_, Tournament>(&format!("UPDATE tournaments SET name=$2, description=$3, start_date=$4, end_date=$5 WHERE id=$1 RETURNING {COLUMNS}"))
            .bind(tournament_id).bind(input.name).bind(input.description).bind(input.start_date).bind(input.end_date)
            .fetch_one(&mut *tx).await?
    } else {
        tournament
    };
    // Check natural expiry after every possible lock/trigger wait, including no-op.
    if auth::lock_active_session(&mut tx, session_id)
        .await?
        .is_none()
    {
        return Err(AuthorizationError::Unauthenticated.into());
    }
    tx.commit().await?;
    Ok(DetailsResult {
        tournament,
        changed,
    })
}
