mod configuration;
mod pairings;

#[cfg(test)]
mod tests;

use uuid::Uuid;

use super::models::{
    PairingValidation, ParticipantStatus, ReadinessIssue, ReadinessIssueCode, RoundStatus,
    ScoringFormat, TournamentStatus,
};

#[derive(Debug, Clone)]
pub struct EntrantFact {
    pub player_id: Uuid,
    pub display_name: String,
    pub participant_status: ParticipantStatus,
    pub player_active: bool,
    pub handicap_index_tenths: i32,
}

#[derive(Debug, Clone)]
pub struct TeamFact {
    pub team_id: Uuid,
    pub team_name: String,
    pub player_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct FlightFact {
    pub flight_id: Uuid,
    pub flight_name: String,
    pub player_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ConfigurationFact {
    pub course_id: Option<Uuid>,
    pub tee_id: Option<Uuid>,
    pub tee_course_id: Option<Uuid>,
    pub slope_rating: Option<i16>,
    pub course_rating_tenths: Option<i32>,
    pub hole_numbers: Vec<i16>,
    pub stroke_indexes: Vec<i16>,
}

#[derive(Debug, Clone)]
pub struct ReadinessFacts {
    pub round_id: Uuid,
    pub round_status: RoundStatus,
    pub tournament_status: TournamentStatus,
    pub scoring_format: ScoringFormat,
    pub handicap_enabled: bool,
    pub number_of_holes: i16,
    pub entrants: Vec<EntrantFact>,
    pub teams: Vec<TeamFact>,
    pub flights: Vec<FlightFact>,
    pub configuration: ConfigurationFact,
}

pub fn validate(facts: &ReadinessFacts) -> PairingValidation {
    let mut issues = Vec::new();
    push_if(
        &mut issues,
        facts.round_status != RoundStatus::Draft,
        ReadinessIssueCode::RoundNotDraft,
        "round must be draft",
    );
    push_if(
        &mut issues,
        facts.tournament_status != TournamentStatus::Active,
        ReadinessIssueCode::TournamentNotOpenable,
        "tournament must be active",
    );
    let eligible_count = facts
        .entrants
        .iter()
        .filter(|entrant| {
            entrant.participant_status == ParticipantStatus::Active && entrant.player_active
        })
        .count();
    push_if(
        &mut issues,
        eligible_count == 0,
        ReadinessIssueCode::NoActiveEntrants,
        "round requires at least one active entrant",
    );

    let pairing = pairings::validate(facts, &mut issues);
    configuration::validate(facts, &mut issues);
    PairingValidation {
        round_id: facts.round_id,
        ready: issues.is_empty(),
        issues,
        missing_players: pairing.missing_team_players,
        ineligible_players: pairing.ineligible_team_players,
        team_sizes: pairing.team_sizes,
        missing_flight_players: pairing.missing_flight_players,
        ineligible_flight_players: pairing.ineligible_flight_players,
        flight_sizes: pairing.flight_sizes,
        legacy_individual_groups: pairing.legacy_individual_groups,
        split_teams: pairing.split_teams,
    }
}

fn push_if(
    issues: &mut Vec<ReadinessIssue>,
    condition: bool,
    code: ReadinessIssueCode,
    message: &'static str,
) {
    if condition {
        issues.push(ReadinessIssue { code, message });
    }
}
