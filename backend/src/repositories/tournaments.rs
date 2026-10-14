mod archive;
mod complete;
mod counted_rounds;
pub mod details;
mod handicaps;
mod reads;
mod start;

pub use archive::{ArchiveTournamentResult, archive_authorized};
pub use complete::{CompleteTournamentResult, complete_authorized};
pub use counted_rounds::{UpdateCountedRoundsResult, update_counted_rounds_authorized};
pub use handicaps::{change_player_handicap_authorized, list_players, list_players_for_session};
pub use reads::{TournamentReadError, get_for_session, list_for_session, list_mine_for_session};
pub use start::{StartTournamentResult, start_authorized};

use chrono::NaiveDate;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::models::{ScoringMode, Tournament},
    repositories::tournament_authorization::AuthorizationError,
};

pub(super) const COLUMNS: &str = "id, name, description, start_date, end_date, number_of_rounds, counted_rounds, mandatory_round_id, tie_break_policy, status, scoring_mode, created_at, updated_at";

#[derive(Debug, Error)]
pub enum TournamentMutationError {
    #[error("resource not found")]
    NotFound,
    #[error("tournament handicap is locked")]
    HandicapLocked,
    #[error("tournament handicap is unchanged")]
    HandicapUnchanged,
    #[error("counted rounds are outside the tournament round count")]
    CountedRoundsInvalid,
    #[error("mandatory round does not belong to the tournament")]
    MandatoryRoundInvalid,
    #[error("tournament configuration is locked")]
    ConfigurationLocked,
    #[error("tournament configuration has changed")]
    ConfigurationStale,
    #[error("tournament is not ready to start")]
    StartNotReady,
    #[error("tournament cannot be started from its current state")]
    StartInvalidState,
    #[error("tournament has changed")]
    StartStale,
    #[error("every configured round must be locked")]
    CompletionNotReady,
    #[error("tournament cannot be completed from its current state")]
    CompletionInvalidState,
    #[error("tournament has changed")]
    CompletionStale,
    #[error("tournament cannot be archived from its current state")]
    ArchiveInvalidState,
    #[error("tournament has changed")]
    ArchiveStale,
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn list(pool: &PgPool) -> Result<Vec<Tournament>, sqlx::Error> {
    sqlx::query_as::<_, Tournament>(&format!(
        "SELECT {COLUMNS} FROM tournaments ORDER BY start_date DESC, name, id"
    ))
    .fetch_all(pool)
    .await
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<Tournament>, sqlx::Error> {
    sqlx::query_as::<_, Tournament>(&format!("SELECT {COLUMNS} FROM tournaments WHERE id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    pool: &PgPool,
    name: &str,
    description: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    number_of_rounds: i16,
    counted_rounds: i16,
    scoring_mode: ScoringMode,
) -> Result<Tournament, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments (id, name, description, start_date, end_date, number_of_rounds, counted_rounds, status, scoring_mode) VALUES ($1, $2, $3, $4, $5, $6, $7, 'draft', $8)")
        .bind(id).bind(name.trim()).bind(description.trim()).bind(start_date).bind(end_date)
        .bind(number_of_rounds).bind(counted_rounds).bind(scoring_mode).execute(pool).await?;
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}
