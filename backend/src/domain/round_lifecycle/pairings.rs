use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::{EntrantFact, ReadinessFacts, push_if};
use crate::domain::models::{
    ParticipantStatus, ReadinessFlightSize, ReadinessIssue, ReadinessIssueCode, ReadinessPlayer,
    ReadinessTeamSize, ScoringFormat,
};

pub(super) struct PairingDetails {
    pub missing_team_players: Vec<ReadinessPlayer>,
    pub ineligible_team_players: Vec<ReadinessPlayer>,
    pub team_sizes: Vec<ReadinessTeamSize>,
    pub missing_flight_players: Vec<ReadinessPlayer>,
    pub ineligible_flight_players: Vec<ReadinessPlayer>,
    pub flight_sizes: Vec<ReadinessFlightSize>,
    pub legacy_individual_groups: Vec<ReadinessTeamSize>,
    pub split_teams: Vec<ReadinessTeamSize>,
}

pub(super) fn validate(facts: &ReadinessFacts, issues: &mut Vec<ReadinessIssue>) -> PairingDetails {
    let eligible: HashMap<Uuid, &EntrantFact> = facts
        .entrants
        .iter()
        .filter(|entrant| {
            entrant.participant_status == ParticipantStatus::Active && entrant.player_active
        })
        .map(|entrant| (entrant.player_id, entrant))
        .collect();
    let entrant_by_id: HashMap<Uuid, &EntrantFact> = facts
        .entrants
        .iter()
        .map(|entrant| (entrant.player_id, entrant))
        .collect();

    let flight_assigned: HashSet<Uuid> = facts
        .flights
        .iter()
        .flat_map(|flight| flight.player_ids.iter().copied())
        .collect();
    let missing_flight_players = sorted_players(
        eligible
            .values()
            .filter(|entrant| !flight_assigned.contains(&entrant.player_id))
            .copied(),
    );
    let ineligible_flight_players =
        assigned_ineligible(&flight_assigned, &eligible, &entrant_by_id);
    let flight_sizes = sorted_flight_sizes(facts);
    push_if(
        issues,
        !missing_flight_players.is_empty(),
        ReadinessIssueCode::MissingFlightAssignment,
        "every active entrant must be assigned to a flight",
    );
    push_if(
        issues,
        !ineligible_flight_players.is_empty(),
        ReadinessIssueCode::IneligibleFlightAssignment,
        "withdrawn or inactive players cannot be assigned to flights",
    );
    push_if(
        issues,
        flight_sizes.iter().any(|flight| flight.player_count == 0),
        ReadinessIssueCode::EmptyFlight,
        "flights cannot be empty",
    );

    let all_team_sizes = sorted_team_sizes(facts);
    let mut missing_team_players = Vec::new();
    let mut ineligible_team_players = Vec::new();
    let team_sizes = all_team_sizes;
    let mut legacy_individual_groups = Vec::new();
    let mut split_teams = Vec::new();
    match facts.scoring_format {
        ScoringFormat::IndividualStrokePlay => {
            legacy_individual_groups = team_sizes.clone();
            push_if(
                issues,
                !legacy_individual_groups.is_empty(),
                ReadinessIssueCode::LegacyIndividualGroupsPresent,
                "individual rounds cannot retain legacy grouping teams",
            );
        }
        ScoringFormat::TeamScramble => {
            let team_assigned: HashSet<Uuid> = facts
                .teams
                .iter()
                .flat_map(|team| team.player_ids.iter().copied())
                .collect();
            missing_team_players = sorted_players(
                eligible
                    .values()
                    .filter(|entrant| !team_assigned.contains(&entrant.player_id))
                    .copied(),
            );
            ineligible_team_players =
                assigned_ineligible(&team_assigned, &eligible, &entrant_by_id);
            push_if(
                issues,
                !missing_team_players.is_empty(),
                ReadinessIssueCode::MissingTeamAssignment,
                "every active entrant must be assigned to a scramble team",
            );
            push_if(
                issues,
                !ineligible_team_players.is_empty(),
                ReadinessIssueCode::IneligibleTeamAssignment,
                "withdrawn or inactive players cannot be assigned to teams",
            );
            push_if(
                issues,
                team_sizes.iter().any(|team| team.player_count == 0),
                ReadinessIssueCode::EmptyTeam,
                "scramble teams cannot be empty",
            );
            push_if(
                issues,
                team_sizes.iter().any(|team| team.player_count != 2),
                ReadinessIssueCode::InvalidScrambleTeamSize,
                "scramble teams must contain exactly two players",
            );
            split_teams = find_split_teams(facts, &team_sizes);
            push_if(
                issues,
                !split_teams.is_empty(),
                ReadinessIssueCode::TeamSplitAcrossFlights,
                "each scramble team must be contained within one flight",
            );
        }
    }

    PairingDetails {
        missing_team_players,
        ineligible_team_players,
        team_sizes,
        missing_flight_players,
        ineligible_flight_players,
        flight_sizes,
        legacy_individual_groups,
        split_teams,
    }
}

fn assigned_ineligible(
    assigned: &HashSet<Uuid>,
    eligible: &HashMap<Uuid, &EntrantFact>,
    entrant_by_id: &HashMap<Uuid, &EntrantFact>,
) -> Vec<ReadinessPlayer> {
    sorted_players(
        assigned
            .iter()
            .filter(|player_id| !eligible.contains_key(player_id))
            .filter_map(|player_id| entrant_by_id.get(player_id).copied()),
    )
}

fn sorted_players<'a>(entrants: impl Iterator<Item = &'a EntrantFact>) -> Vec<ReadinessPlayer> {
    let mut players = entrants
        .map(|entrant| ReadinessPlayer {
            player_id: entrant.player_id,
            display_name: entrant.display_name.clone(),
        })
        .collect::<Vec<_>>();
    players.sort_by(|a, b| {
        a.display_name
            .cmp(&b.display_name)
            .then(a.player_id.cmp(&b.player_id))
    });
    players
}

fn sorted_team_sizes(facts: &ReadinessFacts) -> Vec<ReadinessTeamSize> {
    let mut sizes = facts
        .teams
        .iter()
        .map(|team| ReadinessTeamSize {
            team_id: team.team_id,
            team_name: team.team_name.clone(),
            player_count: team.player_ids.len(),
        })
        .collect::<Vec<_>>();
    sizes.sort_by(|a, b| {
        a.team_name
            .cmp(&b.team_name)
            .then(a.team_id.cmp(&b.team_id))
    });
    sizes
}

fn sorted_flight_sizes(facts: &ReadinessFacts) -> Vec<ReadinessFlightSize> {
    let mut sizes = facts
        .flights
        .iter()
        .map(|flight| ReadinessFlightSize {
            flight_id: flight.flight_id,
            flight_name: flight.flight_name.clone(),
            player_count: flight.player_ids.len(),
        })
        .collect::<Vec<_>>();
    sizes.sort_by(|a, b| {
        a.flight_name
            .cmp(&b.flight_name)
            .then(a.flight_id.cmp(&b.flight_id))
    });
    sizes
}

fn find_split_teams(
    facts: &ReadinessFacts,
    team_sizes: &[ReadinessTeamSize],
) -> Vec<ReadinessTeamSize> {
    let mut flights_by_player: HashMap<Uuid, Uuid> = HashMap::new();
    for flight in &facts.flights {
        for player_id in &flight.player_ids {
            flights_by_player.insert(*player_id, flight.flight_id);
        }
    }
    let sizes_by_id: HashMap<Uuid, &ReadinessTeamSize> =
        team_sizes.iter().map(|size| (size.team_id, size)).collect();
    let mut split = facts
        .teams
        .iter()
        .filter_map(|team| {
            let flight_ids = team
                .player_ids
                .iter()
                .filter_map(|player_id| flights_by_player.get(player_id))
                .collect::<HashSet<_>>();
            (flight_ids.len() > 1)
                .then(|| sizes_by_id.get(&team.team_id).copied())
                .flatten()
                .cloned()
        })
        .collect::<Vec<_>>();
    split.sort_by(|a, b| {
        a.team_name
            .cmp(&b.team_name)
            .then(a.team_id.cmp(&b.team_id))
    });
    split
}
