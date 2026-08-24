use sqlx::PgConnection;

use crate::domain::{
    round_formats::{RoundFormatPolicy, TeamPlayingHandicap},
    scorecards::ScoreOwner,
    scoring::ScoringError::InvalidTeamSize,
};

use super::{ScorecardConflict, ScorecardError, rows::RoundContext};

pub(super) async fn validate_owner(
    connection: &mut PgConnection,
    context: &RoundContext,
    owner: ScoreOwner,
) -> Result<i32, ScorecardError> {
    match (RoundFormatPolicy::for_format(context.scoring_format), owner) {
        (RoundFormatPolicy::PlayerOwned { .. }, ScoreOwner::Player { id }) => {
            let handicap = sqlx::query_scalar::<_, i16>(
                "SELECT playing_handicap FROM round_handicap_snapshots WHERE round_id = $1 AND player_id = $2",
            )
            .bind(context.id)
            .bind(id)
            .fetch_optional(&mut *connection)
            .await?;
            if let Some(handicap) = handicap {
                return Ok(if context.handicap_enabled {
                    i32::from(handicap)
                } else {
                    0
                });
            }
            player_missing_or_ineligible(connection, id).await
        }
        (
            RoundFormatPolicy::TeamOwned {
                exact_team_size,
                team_playing_handicap,
                ..
            },
            ScoreOwner::Team { id },
        ) => {
            let team_round =
                sqlx::query_scalar::<_, uuid::Uuid>("SELECT round_id FROM teams WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&mut *connection)
                    .await?;
            let Some(team_round) = team_round else {
                return Err(ScorecardError::NotFound);
            };
            if team_round != context.id {
                return Err(ScorecardError::Conflict(
                    ScorecardConflict::OwnerNotEligible,
                ));
            }
            let member_count = sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM team_memberships WHERE round_id = $1 AND team_id = $2",
            )
            .bind(context.id)
            .bind(id)
            .fetch_one(&mut *connection)
            .await?;
            let handicaps = sqlx::query_scalar::<_, i16>(
                "SELECT rhs.course_handicap FROM team_memberships tm JOIN round_handicap_snapshots rhs ON rhs.round_id = tm.round_id AND rhs.player_id = tm.player_id WHERE tm.team_id = $1 ORDER BY rhs.course_handicap, rhs.player_id",
            )
            .bind(id)
            .fetch_all(&mut *connection)
            .await?;
            let preserved_team_handicap = if team_playing_handicap.uses_preserved_team_snapshot() {
                sqlx::query_scalar::<_, i16>(
                    "SELECT playing_handicap FROM round_team_handicap_snapshots WHERE round_id = $1 AND team_id = $2",
                )
                .bind(context.id)
                .bind(id)
                .fetch_optional(&mut *connection)
                .await?
            } else {
                None
            };
            team_owner_playing_handicap(
                exact_team_size,
                team_playing_handicap,
                member_count,
                &handicaps,
                context.handicap_allowance_percent,
                context.handicap_enabled,
                preserved_team_handicap,
            )
        }
        _ => Err(ScorecardError::Conflict(
            ScorecardConflict::OwnerFormatMismatch,
        )),
    }
}

fn team_owner_playing_handicap(
    exact_team_size: u16,
    formula: TeamPlayingHandicap,
    member_count: i64,
    course_handicaps: &[i16],
    allowance_percent: i16,
    handicap_enabled: bool,
    preserved_team_handicap: Option<i16>,
) -> Result<i32, ScorecardError> {
    if member_count != i64::from(exact_team_size)
        || course_handicaps.len() != usize::from(exact_team_size)
    {
        return Err(ScorecardError::Scoring(InvalidTeamSize));
    }
    let calculated = if formula.uses_preserved_team_snapshot() {
        preserved_team_handicap
            .map(i32::from)
            .ok_or(ScorecardError::InvalidStoredData)?
    } else {
        formula
            .calculate(
                &course_handicaps
                    .iter()
                    .copied()
                    .map(i32::from)
                    .collect::<Vec<_>>(),
                allowance_percent,
            )?
            .ok_or(ScorecardError::InvalidStoredData)?
    };
    Ok(if handicap_enabled { calculated } else { 0 })
}

async fn player_missing_or_ineligible(
    connection: &mut PgConnection,
    id: uuid::Uuid,
) -> Result<i32, ScorecardError> {
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM players WHERE id = $1)")
            .bind(id)
            .fetch_one(connection)
            .await?;
    if exists {
        Err(ScorecardError::Conflict(
            ScorecardConflict::OwnerNotEligible,
        ))
    } else {
        Err(ScorecardError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_handicap_does_not_hide_an_invalid_team_size() {
        let result = team_owner_playing_handicap(
            2,
            TeamPlayingHandicap::Scramble35And15,
            3,
            &[10, 20],
            95,
            false,
            None,
        );

        assert!(matches!(
            result,
            Err(ScorecardError::Scoring(
                crate::domain::scoring::ScoringError::InvalidTeamSize
            ))
        ));
    }
}
