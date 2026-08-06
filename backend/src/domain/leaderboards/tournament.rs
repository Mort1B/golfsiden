use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::domain::models::RoundStatus;

use super::{
    CurrentTeam, LeaderboardError, LeaderboardMetric, LeaderboardOwner, ParticipantFact,
    TournamentLeaderboard, TournamentLeaderboardEntry, TournamentLeaderboardFacts,
    build_round_leaderboard, round::name_cmp,
};

pub fn build_tournament_leaderboard(
    facts: &TournamentLeaderboardFacts,
    metric: LeaderboardMetric,
) -> Result<TournamentLeaderboard, LeaderboardError> {
    let mut participants = HashMap::new();
    for participant in &facts.participants {
        if participants
            .insert(participant.player_id, participant)
            .is_some()
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
    }
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
    let mut totals: HashMap<uuid::Uuid, (i32, i32, usize)> = HashMap::new();
    let mut attributed = HashSet::new();
    for round in &included {
        let leaderboard = built_rounds
            .remove(&round.round.round_id)
            .ok_or(LeaderboardError::InvalidStoredData)?;
        for entry in leaderboard.entries {
            if !entry.complete {
                return Err(LeaderboardError::InvalidStoredData);
            }
            match entry.owner {
                LeaderboardOwner::Player { id } => {
                    add_result(
                        &participants,
                        &mut totals,
                        &mut attributed,
                        round.round.round_id,
                        id,
                        entry.gross_total,
                        entry.net_total,
                    )?;
                }
                LeaderboardOwner::Team { .. } => {
                    for member in entry.members {
                        add_result(
                            &participants,
                            &mut totals,
                            &mut attributed,
                            round.round.round_id,
                            member.player_id,
                            entry.gross_total,
                            entry.net_total,
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
            let (gross_total, net_total, completed_rounds) =
                totals.remove(&participant.player_id).unwrap_or((0, 0, 0));
            TournamentLeaderboardEntry {
                position: None,
                tied: false,
                player_id: participant.player_id,
                display_name: participant.display_name.clone(),
                status: participant.status,
                completed_rounds,
                gross_total,
                net_total,
                current_team: current_teams.get(&participant.player_id).cloned(),
            }
        })
        .collect::<Vec<_>>();
    rank_entries(&mut entries, metric);
    Ok(TournamentLeaderboard {
        tournament_id: facts.tournament_id,
        metric,
        current_round_id: current_round.map(|round| round.round.round_id),
        included_round_ids: included.iter().map(|round| round.round.round_id).collect(),
        entries,
    })
}

fn current_teams(
    round: &super::RoundLeaderboardFacts,
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

#[allow(clippy::too_many_arguments)]
fn add_result(
    participants: &HashMap<uuid::Uuid, &ParticipantFact>,
    totals: &mut HashMap<uuid::Uuid, (i32, i32, usize)>,
    attributed: &mut HashSet<(uuid::Uuid, uuid::Uuid)>,
    round_id: uuid::Uuid,
    player_id: uuid::Uuid,
    gross: i32,
    net: i32,
) -> Result<(), LeaderboardError> {
    if !participants.contains_key(&player_id) || !attributed.insert((round_id, player_id)) {
        return Err(LeaderboardError::InvalidStoredData);
    }
    let total = totals.entry(player_id).or_default();
    total.0 += gross;
    total.1 += net;
    total.2 += 1;
    Ok(())
}

fn rank_entries(entries: &mut [TournamentLeaderboardEntry], metric: LeaderboardMetric) {
    entries.sort_by(
        |left, right| match (left.completed_rounds > 0, right.completed_rounds > 0) {
            (true, true) => right
                .completed_rounds
                .cmp(&left.completed_rounds)
                .then(selected_total(left, metric).cmp(&selected_total(right, metric)))
                .then_with(|| name_cmp(&left.display_name, &right.display_name))
                .then(left.player_id.cmp(&right.player_id)),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => name_cmp(&left.display_name, &right.display_name)
                .then(left.player_id.cmp(&right.player_id)),
        },
    );
    let ranked = entries
        .iter()
        .take_while(|entry| entry.completed_rounds > 0)
        .count();
    for index in 0..ranked {
        let tied_previous =
            index > 0 && rank_key(&entries[index - 1], metric) == rank_key(&entries[index], metric);
        entries[index].position = Some(if tied_previous {
            entries[index - 1].position.unwrap_or(index)
        } else {
            index + 1
        });
        entries[index].tied = tied_previous
            || entries
                .get(index + 1)
                .is_some_and(|next| rank_key(next, metric) == rank_key(&entries[index], metric));
    }
}

fn rank_key(entry: &TournamentLeaderboardEntry, metric: LeaderboardMetric) -> (usize, i32) {
    (entry.completed_rounds, selected_total(entry, metric))
}

fn selected_total(entry: &TournamentLeaderboardEntry, metric: LeaderboardMetric) -> i32 {
    match metric {
        LeaderboardMetric::Gross => entry.gross_total,
        LeaderboardMetric::Net => entry.net_total,
    }
}
