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
        teams: match format {
            ScoringFormat::IndividualStrokePlay => Vec::new(),
            ScoringFormat::TeamScramble | ScoringFormat::TwoPlayerFoursomes => vec![TeamFact {
                team_id: Uuid::from_u128(20),
                team_name: "Team 1".to_owned(),
                player_ids: vec![player_a, player_b],
            }],
        },
        flights: vec![FlightFact {
            flight_id: Uuid::from_u128(21),
            flight_name: "Flight 1".to_owned(),
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

fn codes(validation: &PairingValidation) -> Vec<ReadinessIssueCode> {
    validation.issues.iter().map(|issue| issue.code).collect()
}

#[test]
fn draft_parent_tournament_is_not_openable() {
    let mut facts = base_facts(ScoringFormat::IndividualStrokePlay);
    facts.tournament_status = TournamentStatus::Draft;

    let validation = validate(&facts);

    assert!(!validation.ready);
    assert!(codes(&validation).contains(&ReadinessIssueCode::TournamentNotOpenable));
}

#[test]
fn individual_requires_complete_flights_and_no_legacy_teams() {
    let mut facts = base_facts(ScoringFormat::IndividualStrokePlay);
    assert!(validate(&facts).ready);
    facts.flights[0].player_ids.pop();
    let missing = validate(&facts);
    assert!(codes(&missing).contains(&ReadinessIssueCode::MissingFlightAssignment));
    assert_eq!(missing.missing_flight_players[0].display_name, "B");
    assert!(missing.missing_players.is_empty());
    facts.teams.push(TeamFact {
        team_id: Uuid::from_u128(40),
        team_name: "Legacy".into(),
        player_ids: vec![Uuid::from_u128(1)],
    });
    let legacy = validate(&facts);
    assert!(codes(&legacy).contains(&ReadinessIssueCode::LegacyIndividualGroupsPresent));
    assert_eq!(legacy.legacy_individual_groups[0].team_name, "Legacy");
    assert_eq!(legacy.team_sizes, legacy.legacy_individual_groups);
}

#[test]
fn flights_reject_empty_and_effectively_ineligible_members() {
    let mut facts = base_facts(ScoringFormat::IndividualStrokePlay);
    facts.entrants[1].player_active = false;
    facts.flights.push(FlightFact {
        flight_id: Uuid::from_u128(22),
        flight_name: "Empty".into(),
        player_ids: vec![],
    });
    let validation = validate(&facts);
    let codes = codes(&validation);
    assert!(codes.contains(&ReadinessIssueCode::IneligibleFlightAssignment));
    assert!(codes.contains(&ReadinessIssueCode::EmptyFlight));
    assert_eq!(validation.ineligible_flight_players[0].display_name, "B");
    assert_eq!(validation.flight_sizes[0].flight_name, "Empty");
}

#[test]
fn scramble_requires_exact_teams_inside_one_flight() {
    let mut facts = base_facts(ScoringFormat::TeamScramble);
    assert!(validate(&facts).ready);
    facts.flights = vec![
        FlightFact {
            flight_id: Uuid::from_u128(21),
            flight_name: "A".into(),
            player_ids: vec![Uuid::from_u128(1)],
        },
        FlightFact {
            flight_id: Uuid::from_u128(22),
            flight_name: "B".into(),
            player_ids: vec![Uuid::from_u128(2)],
        },
    ];
    let split = validate(&facts);
    assert!(codes(&split).contains(&ReadinessIssueCode::TeamSplitAcrossFlights));
    assert_eq!(split.split_teams[0].team_name, "Team 1");
    facts.teams[0].player_ids.pop();
    let incomplete = validate(&facts);
    assert!(codes(&incomplete).contains(&ReadinessIssueCode::MissingTeamAssignment));
    assert!(codes(&incomplete).contains(&ReadinessIssueCode::InvalidScrambleTeamSize));
}

#[test]
fn missing_scramble_flight_assignment_is_not_reported_as_a_split_team() {
    let mut facts = base_facts(ScoringFormat::TeamScramble);
    facts.flights[0].player_ids.pop();
    let validation = validate(&facts);
    assert!(codes(&validation).contains(&ReadinessIssueCode::MissingFlightAssignment));
    assert!(!codes(&validation).contains(&ReadinessIssueCode::TeamSplitAcrossFlights));
    assert!(validation.split_teams.is_empty());
    assert_eq!(validation.team_sizes[0].player_count, 2);
}

#[test]
fn multiple_scramble_teams_may_share_one_flight_and_schedules_are_absent() {
    let mut facts = base_facts(ScoringFormat::TeamScramble);
    for value in 3..=4 {
        facts.entrants.push(EntrantFact {
            player_id: Uuid::from_u128(value),
            display_name: format!("P{value}"),
            participant_status: ParticipantStatus::Active,
            player_active: true,
            handicap_index_tenths: 100,
        });
        facts.flights[0].player_ids.push(Uuid::from_u128(value));
    }
    facts.teams.push(TeamFact {
        team_id: Uuid::from_u128(23),
        team_name: "Team 2".into(),
        player_ids: vec![Uuid::from_u128(3), Uuid::from_u128(4)],
    });
    assert!(validate(&facts).ready);
}

#[test]
fn foursomes_reuses_exact_two_player_team_and_flight_readiness() {
    let mut facts = base_facts(ScoringFormat::TwoPlayerFoursomes);
    assert!(validate(&facts).ready);

    facts.teams[0].player_ids.pop();
    let validation = validate(&facts);
    assert!(!validation.ready);
    assert!(codes(&validation).contains(&ReadinessIssueCode::InvalidFoursomesTeamSize));
    assert!(
        validation
            .issues
            .iter()
            .all(|issue| !issue.message.contains("scramble"))
    );
    assert!(validation.issues.iter().any(|issue| {
        issue.code == ReadinessIssueCode::InvalidFoursomesTeamSize
            && issue.message == "foursomes teams must contain exactly two players"
    }));
}

#[test]
fn readiness_rejects_shifted_configuration_facts() {
    let mut facts = base_facts(ScoringFormat::IndividualStrokePlay);
    facts.configuration.hole_numbers = vec![2, 3];
    facts.configuration.stroke_indexes = vec![2, 3];
    facts.configuration.slope_rating = None;
    let codes = codes(&validate(&facts));
    assert!(codes.contains(&ReadinessIssueCode::InvalidHoleNumbers));
    assert!(codes.contains(&ReadinessIssueCode::InvalidStrokeIndexes));
    assert!(codes.contains(&ReadinessIssueCode::MissingHandicapRatings));
}
