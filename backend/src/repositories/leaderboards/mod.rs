mod load;
mod rows;

use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::leaderboards::{
    LeaderboardError as DomainError, LeaderboardMetric, ParticipantFact, RoundFact,
    RoundLeaderboard, TournamentLeaderboard, TournamentLeaderboardFacts, build_round_leaderboard,
    build_round_leaderboard_projected, build_tournament_leaderboard,
    build_tournament_leaderboard_projected,
};
use crate::domain::{
    models::TournamentRole,
    score_visibility::{
        VisibilityFacts, VisibilityMetadata, VisibilityMode, unrestricted, visibility,
    },
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
        "SELECT r.id AS round_id, r.tournament_id, r.round_number, r.status, r.scoring_format, r.number_of_holes, r.handicap_enabled, r.handicap_allowance_percent, r.final_scores_hidden_until, t.number_of_rounds AS tournament_round_count FROM rounds r JOIN tournaments t ON t.id = r.tournament_id WHERE r.id = $1",
    )
    .bind(round_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(LeaderboardError::NotFound)?;
    let observed_at = sqlx::query_scalar("SELECT transaction_timestamp()")
        .fetch_one(&mut *transaction)
        .await?;
    let role = if let Some(user_id) = user_id {
        tournament_authorization::require_tournament_member_read(
            &mut transaction,
            user_id,
            row.tournament_id,
        )
        .await?;
        Some(member_role(&mut transaction, user_id, row.tournament_id).await?)
    } else {
        None
    };
    let projection = projection_for_round(&row, role, observed_at);
    let round = round_from_row(row);
    let facts = load::related(&mut transaction, vec![round])
        .await?
        .pop()
        .ok_or(LeaderboardError::InvalidStoredData)?;
    let result = match role {
        Some(_) => build_round_leaderboard_projected(&facts, metric, projection)?,
        None => build_round_leaderboard(&facts, metric)?,
    };
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
    let (counted_rounds, mandatory_round_id) = sqlx::query_as::<_, (i16, Option<Uuid>)>(
        "SELECT counted_rounds, mandatory_round_id FROM tournaments WHERE id = $1",
    )
    .bind(tournament_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(LeaderboardError::NotFound)?;
    let counted_rounds = usize::try_from(counted_rounds)
        .ok()
        .filter(|count| *count > 0)
        .ok_or(LeaderboardError::InvalidStoredData)?;
    let observed_at = sqlx::query_scalar("SELECT transaction_timestamp()")
        .fetch_one(&mut *transaction)
        .await?;
    let role = if let Some(user_id) = user_id {
        tournament_authorization::require_tournament_member_read(
            &mut transaction,
            user_id,
            tournament_id,
        )
        .await?;
        Some(member_role(&mut transaction, user_id, tournament_id).await?)
    } else {
        None
    };
    let round_rows = sqlx::query_as::<_, RoundRow>(
        "SELECT r.id AS round_id, r.tournament_id, r.round_number, r.status, r.scoring_format, r.number_of_holes, r.handicap_enabled, r.handicap_allowance_percent, r.final_scores_hidden_until, t.number_of_rounds AS tournament_round_count FROM rounds r JOIN tournaments t ON t.id = r.tournament_id WHERE r.tournament_id = $1 AND r.status IN ('open', 'completed', 'locked') ORDER BY r.round_number, r.id",
    )
    .bind(tournament_id)
    .fetch_all(&mut *transaction)
    .await?;
    let final_projection = round_rows
        .iter()
        .find(|row| row.round_number == row.tournament_round_count)
        .map(|row| projection_for_round(row, role, observed_at))
        .unwrap_or_else(|| unrestricted(observed_at));
    let hidden_completed_round_id = round_rows
        .iter()
        .find(|row| {
            row.round_number == row.tournament_round_count
                && matches!(
                    row.status,
                    crate::domain::models::RoundStatus::Completed
                        | crate::domain::models::RoundStatus::Locked
                )
                && final_projection.mode == VisibilityMode::FrontNine
        })
        .map(|row| row.round_id);
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
    let facts = TournamentLeaderboardFacts {
        tournament_id,
        counted_rounds,
        mandatory_round_id,
        participants,
        rounds,
    };
    let result = match role {
        Some(_) => build_tournament_leaderboard_projected(
            &facts,
            metric,
            final_projection,
            hidden_completed_round_id,
        )?,
        None => build_tournament_leaderboard(&facts, metric)?,
    };
    transaction.commit().await?;
    Ok(result)
}

async fn member_role(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<TournamentRole, LeaderboardError> {
    sqlx::query_scalar(
        "SELECT role FROM tournament_memberships WHERE user_id = $1 AND tournament_id = $2 FOR SHARE",
    )
    .bind(user_id)
    .bind(tournament_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(LeaderboardError::InvalidStoredData)
}

fn projection_for_round(
    row: &RoundRow,
    role: Option<TournamentRole>,
    observed_at: chrono::DateTime<chrono::Utc>,
) -> VisibilityMetadata {
    role.map_or_else(
        || unrestricted(observed_at),
        |role| {
            visibility(VisibilityFacts {
                role,
                is_final_round: row.round_number == row.tournament_round_count,
                status: row.status,
                number_of_holes: row.number_of_holes,
                hidden_until: row.final_scores_hidden_until,
                observed_at,
            })
        },
    )
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

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::domain::{
        models::{RoundStatus, ScoringFormat},
        score_visibility::VisibilityMode,
    };

    #[test]
    fn repository_projection_uses_exact_observed_deadline_equality() {
        let observed_at = chrono::Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap();
        let mut row = RoundRow {
            round_id: Uuid::from_u128(1),
            tournament_id: Uuid::from_u128(2),
            round_number: 4,
            status: RoundStatus::Completed,
            scoring_format: ScoringFormat::IndividualStrokePlay,
            number_of_holes: 18,
            handicap_enabled: true,
            handicap_allowance_percent: 100,
            final_scores_hidden_until: Some(observed_at),
            tournament_round_count: 4,
        };
        assert_eq!(
            projection_for_round(&row, Some(TournamentRole::Player), observed_at).mode,
            VisibilityMode::Full
        );
        row.final_scores_hidden_until = Some(observed_at + chrono::Duration::nanoseconds(1));
        assert_eq!(
            projection_for_round(&row, Some(TournamentRole::Player), observed_at).mode,
            VisibilityMode::FrontNine
        );
    }
}
