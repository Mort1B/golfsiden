use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::super::{
    CurrentTeam, LeaderboardError, LeaderboardOwner, ParticipantFact, RoundLeaderboard,
    RoundLeaderboardFacts, TournamentContribution,
};

#[derive(Debug, Clone)]
pub(super) struct CandidateContribution {
    pub round_number: i16,
    pub value: TournamentContribution,
}

pub(super) fn participant_map(
    facts: &[ParticipantFact],
) -> Result<HashMap<Uuid, &ParticipantFact>, LeaderboardError> {
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

pub(super) fn current_teams(
    round: &RoundLeaderboardFacts,
    participants: &HashMap<Uuid, &ParticipantFact>,
) -> Result<HashMap<Uuid, CurrentTeam>, LeaderboardError> {
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

pub(super) fn attribute_round(
    participants: &HashMap<Uuid, &ParticipantFact>,
    contributions: &mut HashMap<Uuid, Vec<CandidateContribution>>,
    attributed: &mut HashSet<(Uuid, Uuid)>,
    leaderboard: RoundLeaderboard,
    round_number: i16,
    mandatory_round_id: Option<Uuid>,
    provisional: bool,
) -> Result<(), LeaderboardError> {
    for entry in leaderboard.entries {
        if provisional {
            if entry.holes_scored == 0 {
                continue;
            }
        } else if entry.complete != Some(true) {
            return Err(LeaderboardError::InvalidStoredData);
        }
        let candidate = CandidateContribution {
            round_number,
            value: TournamentContribution {
                round_id: leaderboard.round_id,
                mandatory: mandatory_round_id == Some(leaderboard.round_id),
                provisional,
                owner: entry.owner,
                owner_name: entry.owner_name,
                holes_scored: entry.holes_scored,
                number_of_holes: entry.number_of_holes,
                gross_total: entry.gross_total,
                net_total: entry.net_total,
                par_total: entry.par_played,
                score_to_par: entry.score_to_par,
                counted: false,
            },
        };
        match entry.owner {
            LeaderboardOwner::Player { id } => {
                add(participants, contributions, attributed, id, candidate)?
            }
            LeaderboardOwner::Team { .. } => {
                for member in entry.members {
                    add(
                        participants,
                        contributions,
                        attributed,
                        member.player_id,
                        candidate.clone(),
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn add(
    participants: &HashMap<Uuid, &ParticipantFact>,
    contributions: &mut HashMap<Uuid, Vec<CandidateContribution>>,
    attributed: &mut HashSet<(Uuid, Uuid)>,
    player_id: Uuid,
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
