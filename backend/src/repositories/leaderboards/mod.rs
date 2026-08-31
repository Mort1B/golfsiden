mod load;
mod rows;

use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::leaderboards::{
    LeaderboardError as DomainError, LeaderboardMetric, ParticipantFact, RoundFact,
    RoundLeaderboard, TournamentLeaderboard, TournamentLeaderboardFacts, build_round_leaderboard,
    build_tournament_leaderboard,
};
use crate::repositories::tournament_authorization::{self, AuthorizationError};

use rows::{ParticipantRow, RoundRow};

#[derive(Debug, Error)]
pub enum LeaderboardError {
    #[error("resource not found")]
    NotFound,
    #[error("stored leaderboard data is inconsistent")]
    InvalidStoredData,
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

impl From<DomainError> for LeaderboardError {
    fn from(_: DomainError) -> Self {
        Self::InvalidStoredData
    }
}

pub async fn round(
    pool: &PgPool,
    round_id: Uuid,
    metric: LeaderboardMetric,
) -> Result<RoundLeaderboard, LeaderboardError> {
    round_read(pool, None, round_id, metric).await
}

pub async fn round_for_member(
    pool: &PgPool,
    user_id: Uuid,
    round_id: Uuid,
    metric: LeaderboardMetric,
) -> Result<RoundLeaderboard, LeaderboardError> {
    round_read(pool, Some(user_id), round_id, metric).await
}

async fn round_read(
    pool: &PgPool,
    user_id: Option<Uuid>,
    round_id: Uuid,
    metric: LeaderboardMetric,
) -> Result<RoundLeaderboard, LeaderboardError> {
    let mut transaction = pool.begin().await?;
    let transaction_mode = if user_id.is_some() {
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ"
    } else {
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY"
    };
    sqlx::query(transaction_mode)
        .execute(&mut *transaction)
        .await?;
    let row = sqlx::query_as::<_, RoundRow>(
        "SELECT id AS round_id, tournament_id, round_number, status, scoring_format, number_of_holes, handicap_enabled, handicap_allowance_percent FROM rounds WHERE id = $1",
    )
    .bind(round_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(LeaderboardError::NotFound)?;
    if let Some(user_id) = user_id {
        tournament_authorization::require_tournament_member_read(
            &mut transaction,
            user_id,
            row.tournament_id,
        )
        .await?;
    }
    let round = round_from_row(row);
    let facts = load::related(&mut transaction, vec![round])
        .await?
        .pop()
        .ok_or(LeaderboardError::InvalidStoredData)?;
    let result = build_round_leaderboard(&facts, metric)?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn tournament(
    pool: &PgPool,
    tournament_id: Uuid,
    metric: LeaderboardMetric,
) -> Result<TournamentLeaderboard, LeaderboardError> {
    tournament_read(pool, None, tournament_id, metric).await
}

pub async fn tournament_for_member(
    pool: &PgPool,
    user_id: Uuid,
    tournament_id: Uuid,
    metric: LeaderboardMetric,
) -> Result<TournamentLeaderboard, LeaderboardError> {
    tournament_read(pool, Some(user_id), tournament_id, metric).await
}

async fn tournament_read(
    pool: &PgPool,
    user_id: Option<Uuid>,
    tournament_id: Uuid,
    metric: LeaderboardMetric,
) -> Result<TournamentLeaderboard, LeaderboardError> {
    let mut transaction = pool.begin().await?;
    let transaction_mode = if user_id.is_some() {
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ"
    } else {
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY"
    };
    sqlx::query(transaction_mode)
        .execute(&mut *transaction)
        .await?;
    let counted_rounds =
        sqlx::query_scalar::<_, i16>("SELECT counted_rounds FROM tournaments WHERE id = $1")
            .bind(tournament_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(LeaderboardError::NotFound)?;
    let counted_rounds = usize::try_from(counted_rounds)
        .ok()
        .filter(|count| *count > 0)
        .ok_or(LeaderboardError::InvalidStoredData)?;
    if let Some(user_id) = user_id {
        tournament_authorization::require_tournament_member_read(
            &mut transaction,
            user_id,
            tournament_id,
        )
        .await?;
    }
    let round_rows = sqlx::query_as::<_, RoundRow>(
        "SELECT id AS round_id, tournament_id, round_number, status, scoring_format, number_of_holes, handicap_enabled, handicap_allowance_percent FROM rounds WHERE tournament_id = $1 AND status IN ('open', 'completed', 'locked') ORDER BY round_number, id",
    )
    .bind(tournament_id)
    .fetch_all(&mut *transaction)
    .await?;
    let rounds = round_rows.into_iter().map(round_from_row).collect();
    let rounds = load::related(&mut transaction, rounds).await?;
    let participants = sqlx::query_as::<_, ParticipantRow>(
        "SELECT tp.player_id, p.display_name, tp.status FROM tournament_players tp JOIN players p ON p.id = tp.player_id WHERE tp.tournament_id = $1 ORDER BY lower(p.display_name), p.display_name, tp.player_id",
    )
    .bind(tournament_id)
    .fetch_all(&mut *transaction)
    .await?
    .into_iter()
    .map(|row| ParticipantFact {
        player_id: row.player_id,
        display_name: row.display_name,
        status: row.status,
    })
    .collect();
    let result = build_tournament_leaderboard(
        &TournamentLeaderboardFacts {
            tournament_id,
            counted_rounds,
            participants,
            rounds,
        },
        metric,
    )?;
    transaction.commit().await?;
    Ok(result)
}

fn round_from_row(row: RoundRow) -> RoundFact {
    RoundFact {
        round_id: row.round_id,
        tournament_id: row.tournament_id,
        round_number: row.round_number,
        status: row.status,
        scoring_format: row.scoring_format,
        number_of_holes: row.number_of_holes,
        handicap_enabled: row.handicap_enabled,
        handicap_allowance_percent: row.handicap_allowance_percent,
    }
}
