use std::collections::{HashMap, HashSet};

use crate::domain::models::RoundStatus;

use super::super::{
    CurrentTeam, LeaderboardError, LeaderboardMetric, LeaderboardOwner, ParticipantFact,
    TournamentContribution, TournamentLeaderboard, TournamentLeaderboardEntry,
    TournamentLeaderboardFacts, build_round_leaderboard,
};
use super::selection::{CandidateContribution, rank_entries, select_best};

pub fn build_tournament_leaderboard(
    facts: &TournamentLeaderboardFacts,
    metric: LeaderboardMetric,
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
            )
        })
        .collect::<Vec<_>>();
    included.sort_by_key(|round| (round.round.round_number, round.round.round_id));
    let mut contributions: HashMap<uuid::Uuid, Vec<CandidateContribution>> = HashMap::new();
    let mut attributed = HashSet::new();
    let mut round_ids = HashSet::new();
    let mut built_rounds = HashMap::new();
    for round in &facts.rounds {
        if round.round.tournament_id != facts.tournament_id
            || !round_ids.insert(round.round.round_id)
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
        built_rounds.insert(
            round.round.round_id,
            build_round_leaderboard(round, metric)?,
        );
    }
    for round in &included {
        let leaderboard = built_rounds
            .remove(&round.round.round_id)
            .ok_or(LeaderboardError::InvalidStoredData)?;
        for entry in leaderboard.entries {
            if !entry.complete {
                return Err(LeaderboardError::InvalidStoredData);
            }
            let candidate = CandidateContribution {
                round_number: round.round.round_number,
                value: TournamentContribution {
                    round_id: round.round.round_id,
                    mandatory: facts.mandatory_round_id == Some(round.round.round_id),
                    owner: entry.owner,
                    owner_name: entry.owner_name,
                    gross_total: entry.gross_total,
                    net_total: entry.net_total,
                    par_total: entry.par_played,
                    score_to_par: entry.score_to_par,
                    counted: false,
                },
            };
            match entry.owner {
                LeaderboardOwner::Player { id } => add_contribution(
                    &participants,
                    &mut contributions,
                    &mut attributed,
                    id,
                    candidate,
                )?,
                LeaderboardOwner::Team { .. } => {
                    for member in entry.members {
                        add_contribution(
                            &participants,
                            &mut contributions,
                            &mut attributed,
                            member.player_id,
                            candidate.clone(),
                        )?;
                    }
                }
            }
        }
    }

    let mut entries = facts
        .participants
        .iter()
        .map(|participant| {
            let mut candidates = contributions
                .remove(&participant.player_id)
                .unwrap_or_default();
            select_best(
                &mut candidates,
                facts.counted_rounds,
                facts.mandatory_round_id,
                metric,
            );
            let completed_rounds = candidates.len();
            let counted_contributions = candidates.iter().filter(|item| item.value.counted).count();
            let (gross_total, net_total, par_total) = candidates
                .iter()
                .filter(|item| item.value.counted)
                .fold((0, 0, 0), |totals, item| {
                    (
                        totals.0 + item.value.gross_total,
                        totals.1 + item.value.net_total,
                        totals.2 + item.value.par_total,
                    )
                });
            let score_to_par = match metric {
                LeaderboardMetric::Gross => gross_total - par_total,
                LeaderboardMetric::Net => net_total - par_total,
            };
            TournamentLeaderboardEntry {
                position: None,
                tied: false,
                player_id: participant.player_id,
                display_name: participant.display_name.clone(),
                status: participant.status,
                completed_rounds,
                counted_contributions,
                eligible: counted_contributions == facts.counted_rounds,
                gross_total,
                net_total,
                par_total,
                score_to_par,
                contributions: candidates.into_iter().map(|item| item.value).collect(),
                current_team: current_teams.get(&participant.player_id).cloned(),
            }
        })
        .collect::<Vec<_>>();
    rank_entries(&mut entries);
    Ok(TournamentLeaderboard {
        tournament_id: facts.tournament_id,
        metric,
        required_counted_rounds: facts.counted_rounds,
        mandatory_round_id: facts.mandatory_round_id,
        current_round_id: current_round.map(|round| round.round.round_id),
        included_round_ids: included.iter().map(|round| round.round.round_id).collect(),
        entries,
    })
}

fn participant_map(
    facts: &[ParticipantFact],
) -> Result<HashMap<uuid::Uuid, &ParticipantFact>, LeaderboardError> {
    let mut participants = HashMap::new();
    for participant in facts {
        if participants
            .insert(participant.player_id, participant)
            .is_some()
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
    }
    Ok(participants)
}

fn add_contribution(
    participants: &HashMap<uuid::Uuid, &ParticipantFact>,
    contributions: &mut HashMap<uuid::Uuid, Vec<CandidateContribution>>,
    attributed: &mut HashSet<(uuid::Uuid, uuid::Uuid)>,
    player_id: uuid::Uuid,
    contribution: CandidateContribution,
) -> Result<(), LeaderboardError> {
    if !participants.contains_key(&player_id)
        || !attributed.insert((contribution.value.round_id, player_id))
    {
        return Err(LeaderboardError::InvalidStoredData);
    }
    contributions
        .entry(player_id)
        .or_default()
        .push(contribution);
    Ok(())
}

fn current_teams(
    round: &super::super::RoundLeaderboardFacts,
    participants: &HashMap<uuid::Uuid, &ParticipantFact>,
) -> Result<HashMap<uuid::Uuid, CurrentTeam>, LeaderboardError> {
    let mut teams = HashMap::new();
    for team in &round.teams {
        if team.round_id != round.round.round_id
            || teams
                .insert(team.team_id, team.team_name.as_str())
                .is_some()
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
    }
    let mut result = HashMap::new();
    for member in &round.memberships {
        let Some(team_name) = teams.get(&member.team_id) else {
            return Err(LeaderboardError::InvalidStoredData);
        };
        if member.round_id != round.round.round_id
            || !participants.contains_key(&member.player_id)
            || result
                .insert(
                    member.player_id,
                    CurrentTeam {
                        round_id: round.round.round_id,
                        team_id: member.team_id,
                        team_name: (*team_name).to_owned(),
                    },
                )
                .is_some()
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
    }
    Ok(result)
}
