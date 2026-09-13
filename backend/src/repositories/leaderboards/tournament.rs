use super::*;

// Public never inherits a session role and never selects the unrestricted path.
pub(super) enum TournamentProjection {
    Internal,
    Member(Uuid),
    Public,
}

pub(super) async fn assemble(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tournament_id: Uuid,
    metric: LeaderboardMetric,
    projection: TournamentProjection,
) -> Result<TournamentLeaderboard, LeaderboardError> {
    let (counted_rounds, mandatory_round_id, final_round_number, tie_break_policy) = sqlx::query_as::<_, (Option<i16>, Option<Uuid>, i16, TournamentTieBreakPolicy)>(
        "SELECT counted_rounds, mandatory_round_id, number_of_rounds, tie_break_policy FROM tournaments WHERE id = $1",
    )
    .bind(tournament_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(LeaderboardError::NotFound)?;
    let role = if let TournamentProjection::Member(user_id) = projection {
        tournament_authorization::require_tournament_member_read(
            transaction,
            user_id,
            tournament_id,
        )
        .await?;
        Some(member_role(transaction, user_id, tournament_id).await?)
    } else {
        match projection {
            TournamentProjection::Public => Some(TournamentRole::Player),
            TournamentProjection::Internal => None,
            TournamentProjection::Member(_) => return Err(LeaderboardError::InvalidStoredData),
        }
    };
    let counted_rounds =
        usize::try_from(counted_rounds.ok_or(LeaderboardError::OverallUnavailable)?)
            .ok()
            .filter(|count| *count > 0)
            .ok_or(LeaderboardError::InvalidStoredData)?;
    // Draft formats determine overall value units; domain assembly separately
    // restricts current and counted contributions to eligible round statuses.
    let round_rows = sqlx::query_as::<_, RoundRow>(
        "SELECT r.id AS round_id, r.tournament_id, r.round_number, r.status, r.scoring_format, r.number_of_holes, r.handicap_enabled, r.handicap_allowance_percent, t.final_round_back_nine_hidden, t.number_of_rounds AS tournament_round_count FROM rounds r JOIN tournaments t ON t.id = r.tournament_id WHERE r.tournament_id = $1 ORDER BY r.round_number, r.id",
    )
    .bind(tournament_id)
    .fetch_all(&mut **transaction)
    .await?;
    let final_projection = round_rows
        .iter()
        .find(|row| row.round_number == row.tournament_round_count)
        .map(|row| projection_for_round(row, role))
        .unwrap_or_else(unrestricted);
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
    let current_projection = round_rows
        .iter()
        .filter(|row| {
            row.status == crate::domain::models::RoundStatus::Open
                && row.scoring_format.contributes_to_overall()
        })
        .max_by_key(|row| (row.round_number, row.round_id))
        .map(|row| projection_for_round(row, role))
        .unwrap_or_else(unrestricted);
    let rounds = round_rows.into_iter().map(round_from_row).collect();
    let rounds = load::related(transaction, rounds).await?;
    let participants = sqlx::query_as::<_, ParticipantRow>(
        "SELECT tp.player_id, p.display_name, tp.status FROM tournament_players tp JOIN players p ON p.id = tp.player_id WHERE tp.tournament_id = $1 ORDER BY lower(p.display_name), p.display_name, tp.player_id",
    )
    .bind(tournament_id)
    .fetch_all(&mut **transaction)
    .await?
    .into_iter()
    .map(|row| ParticipantFact {
        player_id: row.player_id,
        display_name: row.display_name,
        status: row.status,
    })
    .collect();
    let facts = TournamentLeaderboardFacts {
        final_round_number,
        tie_break_policy,
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
            current_projection,
        )?,
        None => build_tournament_leaderboard(&facts, metric)?,
    };
    Ok(result)
}
