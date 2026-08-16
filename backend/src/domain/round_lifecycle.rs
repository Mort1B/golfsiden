use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::models::{
    PairingValidation, ParticipantStatus, ReadinessIssue, ReadinessIssueCode, ReadinessPlayer,
    ReadinessTeamSize, RoundStatus, ScoringFormat, TournamentStatus,
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
    pub configuration: ConfigurationFact,
}

pub fn validate(facts: &ReadinessFacts) -> PairingValidation {
    let eligible: HashMap<Uuid, &EntrantFact> = facts
        .entrants
        .iter()
        .filter(|entrant| {
            entrant.participant_status == ParticipantStatus::Active && entrant.player_active
        })
        .map(|entrant| (entrant.player_id, entrant))
        .collect();
    let assigned: HashSet<Uuid> = facts
        .teams
        .iter()
        .flat_map(|team| team.player_ids.iter().copied())
        .collect();

    let mut missing_players = eligible
        .values()
        .filter(|entrant| !assigned.contains(&entrant.player_id))
        .map(|entrant| player_ref(entrant))
        .collect::<Vec<_>>();
    let entrant_by_id: HashMap<Uuid, &EntrantFact> = facts
        .entrants
        .iter()
        .map(|entrant| (entrant.player_id, entrant))
        .collect();
    let mut ineligible_players = assigned
        .iter()
        .filter(|player_id| !eligible.contains_key(player_id))
        .filter_map(|player_id| {
            entrant_by_id
                .get(player_id)
                .map(|entrant| player_ref(entrant))
        })
        .collect::<Vec<_>>();
    missing_players.sort_by(|a, b| {
        a.display_name
            .cmp(&b.display_name)
            .then(a.player_id.cmp(&b.player_id))
    });
    ineligible_players.sort_by(|a, b| {
        a.display_name
            .cmp(&b.display_name)
            .then(a.player_id.cmp(&b.player_id))
    });

    let mut team_sizes = facts
        .teams
        .iter()
        .map(|team| ReadinessTeamSize {
            team_id: team.team_id,
            team_name: team.team_name.clone(),
            player_count: team.player_ids.len(),
        })
        .collect::<Vec<_>>();
    team_sizes.sort_by(|a, b| {
        a.team_name
            .cmp(&b.team_name)
            .then(a.team_id.cmp(&b.team_id))
    });

    let mut issues = Vec::new();
    push_if(
        &mut issues,
        facts.round_status != RoundStatus::Draft,
        ReadinessIssueCode::RoundNotDraft,
        "round must be draft",
    );
    push_if(
        &mut issues,
        !matches!(
            facts.tournament_status,
            TournamentStatus::Draft | TournamentStatus::Active
        ),
        ReadinessIssueCode::TournamentNotOpenable,
        "tournament must be draft or active",
    );
    push_if(
        &mut issues,
        eligible.is_empty(),
        ReadinessIssueCode::NoActiveEntrants,
        "round requires at least one active entrant",
    );
    push_if(
        &mut issues,
        !missing_players.is_empty(),
        ReadinessIssueCode::MissingTeamAssignment,
        "every active entrant must be assigned to a group",
    );
    push_if(
        &mut issues,
        !ineligible_players.is_empty(),
        ReadinessIssueCode::IneligibleTeamAssignment,
        "withdrawn or inactive players cannot be assigned",
    );
    push_if(
        &mut issues,
        team_sizes.iter().any(|team| team.player_count == 0),
        ReadinessIssueCode::EmptyTeam,
        "round groups cannot be empty",
    );
    push_if(
        &mut issues,
        facts.scoring_format == ScoringFormat::TeamScramble
            && team_sizes.iter().any(|team| team.player_count != 2),
        ReadinessIssueCode::InvalidScrambleTeamSize,
        "scramble teams must contain exactly two players",
    );

    validate_configuration(facts, &mut issues);
    PairingValidation {
        round_id: facts.round_id,
        ready: issues.is_empty(),
        issues,
        missing_players,
        ineligible_players,
        team_sizes,
    }
}

fn validate_configuration(facts: &ReadinessFacts, issues: &mut Vec<ReadinessIssue>) {
    let configuration = &facts.configuration;
    push_if(
        issues,
        configuration.course_id.is_none(),
        ReadinessIssueCode::MissingCourse,
        "round requires a course",
    );
    push_if(
        issues,
        configuration.tee_id.is_none(),
        ReadinessIssueCode::MissingTee,
        "round requires a tee",
    );
    push_if(
        issues,
        configuration.course_id.is_some()
            && configuration.tee_id.is_some()
            && configuration.tee_course_id != configuration.course_id,
        ReadinessIssueCode::MismatchedCourseTee,
        "tee must belong to the round course",
    );
    push_if(
        issues,
        facts.handicap_enabled
            && (configuration.slope_rating.is_none()
                || configuration.course_rating_tenths.is_none()),
        ReadinessIssueCode::MissingHandicapRatings,
        "handicap-enabled rounds require slope and course ratings",
    );
    push_if(
        issues,
        configuration.hole_numbers.len() != facts.number_of_holes as usize,
        ReadinessIssueCode::InvalidHoleCount,
        "tee must have exactly the configured number of holes",
    );
    push_if(
        issues,
        !is_complete_permutation(&configuration.hole_numbers, facts.number_of_holes),
        ReadinessIssueCode::InvalidHoleNumbers,
        "hole numbers must be the complete range for the round",
    );
    push_if(
        issues,
        !is_complete_permutation(&configuration.stroke_indexes, facts.number_of_holes),
        ReadinessIssueCode::InvalidStrokeIndexes,
        "stroke indexes must be the complete range for the round",
    );
}

fn is_complete_permutation(values: &[i16], expected_count: i16) -> bool {
    if expected_count < 1 || values.len() != expected_count as usize {
        return false;
    }
    let unique = values.iter().copied().collect::<HashSet<_>>();
    (1..=expected_count).all(|value| unique.contains(&value)) && unique.len() == values.len()
}

fn player_ref(entrant: &EntrantFact) -> ReadinessPlayer {
    ReadinessPlayer {
        player_id: entrant.player_id,
        display_name: entrant.display_name.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn base_facts(format: ScoringFormat) -> ReadinessFacts {
        let player_a = Uuid::from_u128(1);
        let player_b = Uuid::from_u128(2);
        ReadinessFacts {
            round_id: Uuid::from_u128(10),
            round_status: RoundStatus::Draft,
            tournament_status: TournamentStatus::Active,
            scoring_format: format,
            handicap_enabled: true,
            number_of_holes: 2,
            entrants: vec![
                EntrantFact {
                    player_id: player_a,
                    display_name: "A".to_owned(),
                    participant_status: ParticipantStatus::Active,
                    player_active: true,
                    handicap_index_tenths: 120,
                },
                EntrantFact {
                    player_id: player_b,
                    display_name: "B".to_owned(),
                    participant_status: ParticipantStatus::Active,
                    player_active: true,
                    handicap_index_tenths: 180,
                },
            ],
            teams: vec![TeamFact {
                team_id: Uuid::from_u128(20),
                team_name: "Team 1".to_owned(),
                player_ids: vec![player_a, player_b],
            }],
            configuration: ConfigurationFact {
                course_id: Some(Uuid::from_u128(30)),
                tee_id: Some(Uuid::from_u128(31)),
                tee_course_id: Some(Uuid::from_u128(30)),
                slope_rating: Some(113),
                course_rating_tenths: Some(720),
                hole_numbers: vec![1, 2],
                stroke_indexes: vec![1, 2],
            },
        }
    }

    #[test]
    fn ready_scramble_requires_exactly_two_members() {
        let mut facts = base_facts(ScoringFormat::TeamScramble);
        assert!(validate(&facts).ready);
        facts.teams[0].player_ids.pop();
        let validation = validate(&facts);
        assert!(!validation.ready);
        assert!(
            validation
                .issues
                .iter()
                .any(|issue| { issue.code == ReadinessIssueCode::InvalidScrambleTeamSize })
        );
    }

    #[test]
    fn individual_groups_allow_any_nonzero_size() {
        let mut facts = base_facts(ScoringFormat::IndividualStrokePlay);
        facts.teams[0].player_ids.pop();
        facts.entrants.pop();
        assert!(validate(&facts).ready);
    }

    #[test]
    fn readiness_reports_missing_and_ineligible_players() {
        let mut facts = base_facts(ScoringFormat::IndividualStrokePlay);
        facts.teams[0].player_ids = vec![facts.entrants[1].player_id];
        facts.entrants[1].participant_status = ParticipantStatus::Withdrawn;
        let validation = validate(&facts);
        assert_eq!(validation.missing_players[0].display_name, "A");
        assert_eq!(validation.ineligible_players[0].display_name, "B");
    }

    #[test]
    fn readiness_rejects_shifted_holes_and_stroke_indexes() {
        let mut facts = base_facts(ScoringFormat::TeamScramble);
        facts.configuration.hole_numbers = vec![2, 3];
        facts.configuration.stroke_indexes = vec![2, 3];
        let validation = validate(&facts);
        assert!(
            validation
                .issues
                .iter()
                .any(|issue| issue.code == ReadinessIssueCode::InvalidHoleNumbers)
        );
        assert!(
            validation
                .issues
                .iter()
                .any(|issue| issue.code == ReadinessIssueCode::InvalidStrokeIndexes)
        );
    }

    #[test]
    fn readiness_rejects_empty_groups_and_missing_course_configuration() {
        let mut facts = base_facts(ScoringFormat::IndividualStrokePlay);
        facts.teams[0].player_ids.clear();
        facts.configuration.course_id = None;
        facts.configuration.tee_id = None;
        facts.configuration.tee_course_id = None;
        facts.configuration.slope_rating = None;
        facts.configuration.course_rating_tenths = None;
        facts.configuration.hole_numbers.clear();
        facts.configuration.stroke_indexes.clear();
        let validation = validate(&facts);
        let codes = validation
            .issues
            .iter()
            .map(|issue| issue.code)
            .collect::<Vec<_>>();
        assert!(codes.contains(&ReadinessIssueCode::EmptyTeam));
        assert!(codes.contains(&ReadinessIssueCode::MissingCourse));
        assert!(codes.contains(&ReadinessIssueCode::MissingTee));
        assert!(codes.contains(&ReadinessIssueCode::MissingHandicapRatings));
        assert!(codes.contains(&ReadinessIssueCode::InvalidHoleCount));
    }
}
