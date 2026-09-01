use std::collections::{HashMap, HashSet};

use crate::domain::models::RoundStatus;
use crate::domain::score_visibility::{VisibilityMetadata, unrestricted};

use super::super::{
    LeaderboardError, LeaderboardMetric, TournamentLeaderboard, TournamentLeaderboardEntry,
    TournamentLeaderboardFacts, build_round_leaderboard, build_round_leaderboard_projected,
};
use super::contributions::{attribute_round, current_teams, participant_map};
use super::selection::{completed_qualification, rank_entries, select_displayed};

pub fn build_tournament_leaderboard(
    facts: &TournamentLeaderboardFacts,
    metric: LeaderboardMetric,
) -> Result<TournamentLeaderboard, LeaderboardError> {
    let full = unrestricted(chrono::DateTime::UNIX_EPOCH);
    build_tournament_leaderboard_projected(facts, metric, full, None, full)
}

pub fn build_tournament_leaderboard_projected(
    facts: &TournamentLeaderboardFacts,
    metric: LeaderboardMetric,
    visibility: VisibilityMetadata,
    hidden_completed_round_id: Option<uuid::Uuid>,
    current_visibility: VisibilityMetadata,
) -> Result<TournamentLeaderboard, LeaderboardError> {
    if facts.counted_rounds == 0 {
        return Err(LeaderboardError::InvalidStoredData);
    }
    let participants = participant_map(&facts.participants)?;
    let current_round = facts
        .rounds
        .iter()
        .filter(|round| round.round.status == RoundStatus::Open)
        .max_by_key(|round| (round.round.round_number, round.round.round_id));
    let current_teams = current_round
        .map(|round| current_teams(round, &participants))
        .transpose()?
        .unwrap_or_default();

    let mut included = facts
        .rounds
        .iter()
        .filter(|round| {
            matches!(
                round.round.status,
                RoundStatus::Completed | RoundStatus::Locked
            ) && hidden_completed_round_id != Some(round.round.round_id)
        })
        .collect::<Vec<_>>();
    included.sort_by_key(|round| (round.round.round_number, round.round.round_id));

    let mut validated_rounds = HashMap::new();
    let mut round_ids = HashSet::new();
    for round in &facts.rounds {
        if round.round.tournament_id != facts.tournament_id
            || !round_ids.insert(round.round.round_id)
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
        validated_rounds.insert(
            round.round.round_id,
            build_round_leaderboard(round, metric)?,
        );
    }

    let mut contributions = HashMap::new();
    let mut attributed = HashSet::new();
    for round in &included {
        let leaderboard = validated_rounds
            .remove(&round.round.round_id)
            .ok_or(LeaderboardError::InvalidStoredData)?;
        attribute_round(
            &participants,
            &mut contributions,
            &mut attributed,
            leaderboard,
            round.round.round_number,
            facts.mandatory_round_id,
            false,
        )?;
    }
    if let Some(round) = current_round {
        let projected = build_round_leaderboard_projected(round, metric, current_visibility)?;
        attribute_round(
            &participants,
            &mut contributions,
            &mut attributed,
            projected,
            round.round.round_number,
            facts.mandatory_round_id,
            true,
        )?;
    }

    let mut entries = facts
        .participants
        .iter()
        .map(|participant| {
            let mut candidates = contributions
                .remove(&participant.player_id)
                .unwrap_or_default();
            let completed_rounds = candidates
                .iter()
                .filter(|candidate| !candidate.value.provisional)
                .count();
            let qualification = completed_qualification(
                &candidates,
                facts.counted_rounds,
                facts.mandatory_round_id,
                metric,
            );
            select_displayed(
                &mut candidates,
                facts.counted_rounds,
                facts.mandatory_round_id,
                metric,
            );
            let displayed_count = candidates
                .iter()
                .filter(|candidate| candidate.value.counted)
                .count();
            if displayed_count > qualification.counted.saturating_add(1) {
                return Err(LeaderboardError::InvalidStoredData);
            }
            let (gross_total, net_total, par_total) = candidates
                .iter()
                .filter(|candidate| candidate.value.counted)
                .fold((0, 0, 0), |totals, candidate| {
                    (
                        totals.0 + candidate.value.gross_total,
                        totals.1 + candidate.value.net_total,
                        totals.2 + candidate.value.par_total,
                    )
                });
            let score_to_par = match metric {
                LeaderboardMetric::Gross => gross_total - par_total,
                LeaderboardMetric::Net => net_total - par_total,
            };
            Ok(TournamentLeaderboardEntry {
                position: None,
                tied: false,
                player_id: participant.player_id,
                display_name: participant.display_name.clone(),
                status: participant.status,
                completed_rounds,
                counted_contributions: qualification.counted,
                eligible: qualification.eligible,
                gross_total,
                net_total,
                par_total,
                score_to_par,
                contributions: candidates
                    .into_iter()
                    .map(|candidate| candidate.value)
                    .collect(),
                current_team: current_teams.get(&participant.player_id).cloned(),
            })
        })
        .collect::<Result<Vec<_>, LeaderboardError>>()?;
    rank_entries(&mut entries);
    Ok(TournamentLeaderboard {
        tournament_id: facts.tournament_id,
        metric,
        required_counted_rounds: facts.counted_rounds,
        mandatory_round_id: facts.mandatory_round_id,
        current_round_id: current_round.map(|round| round.round.round_id),
        included_round_ids: included.iter().map(|round| round.round.round_id).collect(),
        visibility,
        entries,
    })
}
