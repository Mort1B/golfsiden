use std::collections::{HashMap, HashSet};

use chrono::NaiveTime;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PairingMemberCommand {
    pub player_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct TeamCommand {
    pub id: Uuid,
    pub name: String,
    pub members: Vec<PairingMemberCommand>,
    pub schedule_flight_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct FlightCommand {
    pub id: Uuid,
    pub name: String,
    pub starting_hole: Option<i16>,
    pub tee_time: Option<NaiveTime>,
    pub members: Vec<PairingMemberCommand>,
}

#[derive(Debug, Clone)]
pub struct LegacyConversionCommand {
    pub team_id: Uuid,
    pub flight_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ReplacementCommand {
    pub teams: Vec<TeamCommand>,
    pub flights: Vec<FlightCommand>,
    pub legacy_conversions: Vec<LegacyConversionCommand>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PairingValidationError {
    #[error("team and flight identifiers must be client-generated UUIDs")]
    InvalidId,
    #[error("team and flight names must be non-empty and trimmed")]
    InvalidName,
    #[error("team names must be unique within the round")]
    DuplicateTeamName,
    #[error("flight names must be unique within the round")]
    DuplicateFlightName,
    #[error("team identifiers must be unique")]
    DuplicateTeamId,
    #[error("flight identifiers must be unique")]
    DuplicateFlightId,
    #[error("a player may appear on at most one team")]
    DuplicateTeamAssignment,
    #[error("a player may appear on at most one flight")]
    DuplicateFlightAssignment,
    #[error("starting_hole must be between 1 and 36")]
    InvalidStartingHole,
    #[error("a team schedule transfer must name a requested flight")]
    UnknownScheduleFlight,
    #[error("legacy mappings must use unique team and flight identifiers")]
    DuplicateLegacyMapping,
}

pub fn validate(command: &ReplacementCommand) -> Result<(), PairingValidationError> {
    let mut team_ids = HashSet::new();
    let mut team_names = HashSet::new();
    let mut team_players = HashSet::new();
    for team in &command.teams {
        if team.id.is_nil() {
            return Err(PairingValidationError::InvalidId);
        }
        validate_name(&team.name)?;
        if !team_ids.insert(team.id) {
            return Err(PairingValidationError::DuplicateTeamId);
        }
        if !team_names.insert(team.name.to_lowercase()) {
            return Err(PairingValidationError::DuplicateTeamName);
        }
        for member in &team.members {
            if !team_players.insert(member.player_id) {
                return Err(PairingValidationError::DuplicateTeamAssignment);
            }
        }
    }

    let mut flight_ids = HashSet::new();
    let mut flight_names = HashSet::new();
    let mut flight_players = HashSet::new();
    for flight in &command.flights {
        if flight.id.is_nil() {
            return Err(PairingValidationError::InvalidId);
        }
        validate_name(&flight.name)?;
        if !flight_ids.insert(flight.id) {
            return Err(PairingValidationError::DuplicateFlightId);
        }
        if !flight_names.insert(flight.name.to_lowercase()) {
            return Err(PairingValidationError::DuplicateFlightName);
        }
        if flight
            .starting_hole
            .is_some_and(|hole| !(1..=36).contains(&hole))
        {
            return Err(PairingValidationError::InvalidStartingHole);
        }
        for member in &flight.members {
            if !flight_players.insert(member.player_id) {
                return Err(PairingValidationError::DuplicateFlightAssignment);
            }
        }
    }
    if command.teams.iter().any(|team| {
        team.schedule_flight_id
            .is_some_and(|flight_id| !flight_ids.contains(&flight_id))
    }) {
        return Err(PairingValidationError::UnknownScheduleFlight);
    }

    let mut legacy_teams = HashSet::new();
    let mut legacy_flights = HashSet::new();
    for mapping in &command.legacy_conversions {
        if !legacy_teams.insert(mapping.team_id) || !legacy_flights.insert(mapping.flight_id) {
            return Err(PairingValidationError::DuplicateLegacyMapping);
        }
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), PairingValidationError> {
    if name.is_empty() || name.trim() != name {
        return Err(PairingValidationError::InvalidName);
    }
    Ok(())
}

pub fn members_by_flight(command: &ReplacementCommand) -> HashMap<Uuid, HashSet<Uuid>> {
    command
        .flights
        .iter()
        .map(|flight| {
            (
                flight.id,
                flight
                    .members
                    .iter()
                    .map(|member| member.player_id)
                    .collect(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(value: u128) -> PairingMemberCommand {
        PairingMemberCommand {
            player_id: Uuid::from_u128(value),
        }
    }

    #[test]
    fn rejects_duplicate_assignments_case_insensitive_names_and_unknown_schedule() {
        let flight_id = Uuid::from_u128(4);
        let mut command = ReplacementCommand {
            teams: vec![
                TeamCommand {
                    id: Uuid::from_u128(1),
                    name: "Alpha".into(),
                    members: vec![member(8)],
                    schedule_flight_id: None,
                },
                TeamCommand {
                    id: Uuid::from_u128(2),
                    name: "alpha".into(),
                    members: vec![member(9)],
                    schedule_flight_id: None,
                },
            ],
            flights: vec![FlightCommand {
                id: flight_id,
                name: "One".into(),
                starting_hole: Some(1),
                tee_time: None,
                members: vec![],
            }],
            legacy_conversions: vec![],
        };
        assert_eq!(
            validate(&command),
            Err(PairingValidationError::DuplicateTeamName)
        );
        command.teams.pop();
        command.teams[0].members.push(member(8));
        assert_eq!(
            validate(&command),
            Err(PairingValidationError::DuplicateTeamAssignment)
        );
        command.teams[0].members.pop();
        command.teams[0].schedule_flight_id = Some(Uuid::from_u128(99));
        assert_eq!(
            validate(&command),
            Err(PairingValidationError::UnknownScheduleFlight)
        );
    }
}
