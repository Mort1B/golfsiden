use std::collections::{HashMap, HashSet};

use crate::domain::{models::ScoringFormat, scoring::scramble_playing_handicap};

use super::{LeaderboardError, LeaderboardMember, LeaderboardOwner, sort_members};
use crate::domain::leaderboards::{RoundLeaderboardFacts, SnapshotFact};

pub(super) struct OwnerSeed<'a> {
    pub(super) owner: LeaderboardOwner,
    pub(super) name: &'a str,
    pub(super) playing_handicap: i32,
    pub(super) members: Vec<LeaderboardMember>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeaderboardFormatPolicy {
    IndividualSnapshots,
    TwoPlayerTeam {
        exact_team_size: usize,
        handicap: TeamHandicapPolicy,
    },
}

impl LeaderboardFormatPolicy {
    fn for_format(format: ScoringFormat) -> Self {
        match format {
            ScoringFormat::IndividualStrokePlay => Self::IndividualSnapshots,
            ScoringFormat::TeamScramble => Self::TwoPlayerTeam {
                exact_team_size: 2,
                handicap: TeamHandicapPolicy::Scramble35And15,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeamHandicapPolicy {
    Scramble35And15,
}

impl TeamHandicapPolicy {
    fn playing_handicap(
        self,
        course_handicaps: &[i32],
        allowance_percent: i16,
    ) -> Result<i32, LeaderboardError> {
        match self {
            Self::Scramble35And15 => scramble_playing_handicap(course_handicaps, allowance_percent)
                .map_err(|_| LeaderboardError::InvalidStoredData),
        }
    }
}

pub(super) fn build_owner_seeds<'a>(
    facts: &'a RoundLeaderboardFacts,
    snapshots: &HashMap<uuid::Uuid, &'a SnapshotFact>,
) -> Result<Vec<OwnerSeed<'a>>, LeaderboardError> {
    match LeaderboardFormatPolicy::for_format(facts.round.scoring_format) {
        LeaderboardFormatPolicy::IndividualSnapshots => {
            Ok(individual_owner_seeds(facts, snapshots))
        }
        LeaderboardFormatPolicy::TwoPlayerTeam {
            exact_team_size,
            handicap,
        } => team_owner_seeds(facts, snapshots, exact_team_size, handicap),
    }
}

fn individual_owner_seeds<'a>(
    facts: &RoundLeaderboardFacts,
    snapshots: &HashMap<uuid::Uuid, &'a SnapshotFact>,
) -> Vec<OwnerSeed<'a>> {
    snapshots
        .iter()
        .map(|(id, snapshot)| OwnerSeed {
            owner: LeaderboardOwner::Player { id: *id },
            name: snapshot.display_name.as_str(),
            playing_handicap: if facts.round.handicap_enabled {
                i32::from(snapshot.playing_handicap)
            } else {
                0
            },
            members: Vec::new(),
        })
        .collect()
}

fn team_owner_seeds<'a>(
    facts: &'a RoundLeaderboardFacts,
    snapshots: &HashMap<uuid::Uuid, &'a SnapshotFact>,
    exact_team_size: usize,
    handicap: TeamHandicapPolicy,
) -> Result<Vec<OwnerSeed<'a>>, LeaderboardError> {
    let mut teams = HashMap::new();
    for team in &facts.teams {
        if team.round_id != facts.round.round_id
            || teams
                .insert(team.team_id, team.team_name.as_str())
                .is_some()
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
    }
    let mut members: HashMap<uuid::Uuid, Vec<LeaderboardMember>> = HashMap::new();
    let mut assigned = HashSet::new();
    for member in &facts.memberships {
        if member.round_id != facts.round.round_id
            || !teams.contains_key(&member.team_id)
            || !assigned.insert(member.player_id)
            || !snapshots.contains_key(&member.player_id)
        {
            return Err(LeaderboardError::InvalidStoredData);
        }
        members
            .entry(member.team_id)
            .or_default()
            .push(LeaderboardMember {
                player_id: member.player_id,
                display_name: member.display_name.clone(),
                display_order: member.display_order,
            });
    }
    if assigned.len() != snapshots.len() {
        return Err(LeaderboardError::InvalidStoredData);
    }

    teams
        .into_iter()
        .map(|(team_id, name)| {
            let mut team_members = members.remove(&team_id).unwrap_or_default();
            sort_members(&mut team_members);
            if team_members.len() != exact_team_size {
                return Err(LeaderboardError::InvalidStoredData);
            }
            let course_handicaps = team_members
                .iter()
                .filter_map(|member| snapshots.get(&member.player_id))
                .map(|snapshot| i32::from(snapshot.course_handicap))
                .collect::<Vec<_>>();
            let calculated = handicap
                .playing_handicap(&course_handicaps, facts.round.handicap_allowance_percent)?;
            Ok(OwnerSeed {
                owner: LeaderboardOwner::Team { id: team_id },
                name,
                playing_handicap: if facts.round.handicap_enabled {
                    calculated
                } else {
                    0
                },
                members: team_members,
            })
        })
        .collect()
}
