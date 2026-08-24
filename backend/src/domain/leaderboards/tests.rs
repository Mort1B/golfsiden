use uuid::Uuid;

use crate::domain::models::{ParticipantStatus, RoundStatus, ScoringFormat};

use super::*;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn round_fact(
    id_value: u128,
    number: i16,
    status: RoundStatus,
    format: ScoringFormat,
) -> RoundFact {
    RoundFact {
        round_id: id(id_value),
        tournament_id: id(100),
        round_number: number,
        status,
        scoring_format: format,
        number_of_holes: 2,
        handicap_enabled: true,
        handicap_allowance_percent: 100,
    }
}

fn holes(round_id: Uuid) -> Vec<HoleFact> {
    vec![
        HoleFact {
            round_id,
            hole_id: id(round_id.as_u128() + 10),
            hole_number: 1,
            par: 4,
            stroke_index: 1,
        },
        HoleFact {
            round_id,
            hole_id: id(round_id.as_u128() + 11),
            hole_number: 2,
            par: 4,
            stroke_index: 2,
        },
    ]
}

fn snapshot(round_id: Uuid, player: u128, name: &str, course: i16, playing: i16) -> SnapshotFact {
    SnapshotFact {
        round_id,
        player_id: id(player),
        display_name: name.to_owned(),
        course_handicap: course,
        playing_handicap: playing,
    }
}

fn individual_round(id_value: u128, number: i16, status: RoundStatus) -> RoundLeaderboardFacts {
    let round = round_fact(
        id_value,
        number,
        status,
        ScoringFormat::IndividualStrokePlay,
    );
    RoundLeaderboardFacts {
        holes: holes(round.round_id),
        snapshots: vec![
            snapshot(round.round_id, 1, "Ada", 1, 1),
            snapshot(round.round_id, 2, "bob", -1, -1),
        ],
        team_snapshots: Vec::new(),
        teams: Vec::new(),
        memberships: Vec::new(),
        scores: Vec::new(),
        confirmations: Vec::new(),
        round,
    }
}

fn score(round: &RoundLeaderboardFacts, player: u128, hole: usize, gross: i16) -> ScoreFact {
    ScoreFact {
        round_id: round.round.round_id,
        hole_id: round.holes[hole].hole_id,
        player_id: Some(id(player)),
        team_id: None,
        gross_strokes: gross,
    }
}

#[test]
fn gross_and_net_are_separate_and_plus_handicaps_add_strokes() {
    let mut facts = individual_round(200, 1, RoundStatus::Open);
    facts.scores = vec![score(&facts, 1, 0, 5), score(&facts, 2, 1, 4)];
    let gross = build_round_leaderboard(&facts, LeaderboardMetric::Gross).unwrap();
    let net = build_round_leaderboard(&facts, LeaderboardMetric::Net).unwrap();
    assert_eq!(gross.entries[0].owner_name, "bob");
    assert_eq!(
        (gross.entries[0].gross_total, gross.entries[0].net_total),
        (4, 5)
    );
    assert_eq!(net.entries[0].owner_name, "Ada");
    assert_eq!(
        (net.entries[0].gross_total, net.entries[0].net_total),
        (5, 4)
    );
}

#[test]
fn provisional_ties_ignore_holes_played_but_use_them_for_display_order() {
    let mut facts = individual_round(200, 1, RoundStatus::Open);
    facts
        .snapshots
        .push(snapshot(facts.round.round_id, 3, "Charlie", 0, 0));
    facts.scores = vec![
        score(&facts, 1, 0, 4),
        score(&facts, 2, 0, 4),
        score(&facts, 2, 1, 4),
    ];
    let result = build_round_leaderboard(&facts, LeaderboardMetric::Gross).unwrap();
    assert_eq!(result.entries[0].owner_name, "bob");
    assert_eq!(result.entries[0].position, Some(1));
    assert!(result.entries[0].tied);
    assert_eq!(result.entries[1].owner_name, "Ada");
    assert_eq!(result.entries[1].position, Some(1));
    assert_eq!(result.entries[2].owner_name, "Charlie");
    assert_eq!(result.entries[2].position, None);
}

#[test]
fn draft_round_has_no_preserved_owners() {
    let mut facts = individual_round(200, 1, RoundStatus::Draft);
    facts.holes.clear();
    facts.snapshots.clear();
    assert!(
        build_round_leaderboard(&facts, LeaderboardMetric::Net)
            .unwrap()
            .entries
            .is_empty()
    );
}

#[test]
fn scramble_uses_frozen_members_and_formula() {
    let round = round_fact(300, 1, RoundStatus::Open, ScoringFormat::TeamScramble);
    let team_id = id(310);
    let mut facts = RoundLeaderboardFacts {
        holes: holes(round.round_id),
        snapshots: vec![
            snapshot(round.round_id, 1, "Ada", 8, 8),
            snapshot(round.round_id, 2, "Bob", 20, 20),
        ],
        team_snapshots: Vec::new(),
        teams: vec![TeamFact {
            round_id: round.round_id,
            team_id,
            team_name: "Pair".to_owned(),
        }],
        memberships: vec![
            MembershipFact {
                round_id: round.round_id,
                team_id,
                player_id: id(2),
                display_name: "Bob".to_owned(),
                display_order: None,
            },
            MembershipFact {
                round_id: round.round_id,
                team_id,
                player_id: id(1),
                display_name: "Ada".to_owned(),
                display_order: Some(2),
            },
        ],
        scores: Vec::new(),
        confirmations: Vec::new(),
        round,
    };
    facts.scores = vec![ScoreFact {
        round_id: facts.round.round_id,
        hole_id: facts.holes[1].hole_id,
        player_id: None,
        team_id: Some(team_id),
        gross_strokes: 5,
    }];
    let result = build_round_leaderboard(&facts, LeaderboardMetric::Net).unwrap();
    assert_eq!(result.entries[0].playing_handicap, 6);
    assert_eq!(result.entries[0].net_total, 2);
    assert_eq!(result.entries[0].members[0].player_id, id(1));
}

#[test]
fn foursomes_uses_preserved_team_handicap_instead_of_rounded_members() {
    let round = round_fact(320, 1, RoundStatus::Open, ScoringFormat::TwoPlayerFoursomes);
    let team_id = id(321);
    let facts = RoundLeaderboardFacts {
        holes: holes(round.round_id),
        snapshots: vec![
            snapshot(round.round_id, 1, "Ada", 1, 1),
            snapshot(round.round_id, 2, "Bob", 2, 2),
        ],
        team_snapshots: vec![TeamSnapshotFact {
            round_id: round.round_id,
            team_id,
            playing_handicap: 1,
        }],
        teams: vec![TeamFact {
            round_id: round.round_id,
            team_id,
            team_name: "Foursomes pair".to_owned(),
        }],
        memberships: vec![
            member(round.round_id, team_id, 1, "Ada"),
            member(round.round_id, team_id, 2, "Bob"),
        ],
        scores: Vec::new(),
        confirmations: Vec::new(),
        round,
    };

    let result = build_round_leaderboard(&facts, LeaderboardMetric::Net).unwrap();
    assert_eq!(result.entries[0].playing_handicap, 1);
}

#[test]
fn handicap_disabled_scramble_still_requires_exactly_two_frozen_members() {
    let mut round = round_fact(300, 1, RoundStatus::Open, ScoringFormat::TeamScramble);
    round.handicap_enabled = false;
    let team_id = id(310);
    let facts = RoundLeaderboardFacts {
        holes: holes(round.round_id),
        snapshots: vec![snapshot(round.round_id, 1, "Ada", 8, 8)],
        team_snapshots: Vec::new(),
        teams: vec![TeamFact {
            round_id: round.round_id,
            team_id,
            team_name: "Solo".to_owned(),
        }],
        memberships: vec![member(round.round_id, team_id, 1, "Ada")],
        scores: Vec::new(),
        confirmations: Vec::new(),
        round,
    };

    assert_eq!(
        build_round_leaderboard(&facts, LeaderboardMetric::Net),
        Err(LeaderboardError::InvalidStoredData)
    );
}

#[test]
fn tournament_aggregates_changing_team_attribution_and_ranks_round_count_first() {
    let mut first = individual_round(200, 1, RoundStatus::Completed);
    first.scores = vec![
        score(&first, 1, 0, 4),
        score(&first, 1, 1, 4),
        score(&first, 2, 0, 3),
        score(&first, 2, 1, 3),
    ];
    let round = round_fact(300, 2, RoundStatus::Locked, ScoringFormat::TeamScramble);
    let team_id = id(310);
    let second = RoundLeaderboardFacts {
        holes: holes(round.round_id),
        snapshots: vec![
            snapshot(round.round_id, 1, "Ada", 0, 0),
            snapshot(round.round_id, 3, "Cara", 0, 0),
        ],
        team_snapshots: Vec::new(),
        teams: vec![TeamFact {
            round_id: round.round_id,
            team_id,
            team_name: "Changing Pair".to_owned(),
        }],
        memberships: vec![
            member(round.round_id, team_id, 1, "Ada"),
            member(round.round_id, team_id, 3, "Cara"),
        ],
        scores: vec![
            team_score(round.round_id, round.round_id.as_u128() + 10, team_id, 10),
            team_score(round.round_id, round.round_id.as_u128() + 11, team_id, 10),
        ],
        confirmations: Vec::new(),
        round,
    };
    let result = build_tournament_leaderboard(
        &TournamentLeaderboardFacts {
            tournament_id: id(100),
            participants: vec![
                participant(1, "Ada"),
                participant(2, "bob"),
                participant(3, "Cara"),
            ],
            rounds: vec![second, first],
        },
        LeaderboardMetric::Gross,
    )
    .unwrap();
    assert_eq!(result.included_round_ids, vec![id(200), id(300)]);
    assert_eq!(
        (
            result.entries[0].player_id,
            result.entries[0].completed_rounds
        ),
        (id(1), 2)
    );
    assert_eq!(
        (
            result.entries[1].player_id,
            result.entries[1].completed_rounds
        ),
        (id(2), 1)
    );
    assert_eq!(
        (
            result.entries[2].player_id,
            result.entries[2].completed_rounds
        ),
        (id(3), 1)
    );
}

fn participant(player: u128, name: &str) -> ParticipantFact {
    ParticipantFact {
        player_id: id(player),
        display_name: name.to_owned(),
        status: ParticipantStatus::Active,
    }
}

fn member(round_id: Uuid, team_id: Uuid, player: u128, name: &str) -> MembershipFact {
    MembershipFact {
        round_id,
        team_id,
        player_id: id(player),
        display_name: name.to_owned(),
        display_order: None,
    }
}

fn team_score(round_id: Uuid, hole: u128, team_id: Uuid, gross: i16) -> ScoreFact {
    ScoreFact {
        round_id,
        hole_id: id(hole),
        player_id: None,
        team_id: Some(team_id),
        gross_strokes: gross,
    }
}

#[test]
fn tournament_ties_use_only_completed_rounds_and_selected_total() {
    let mut round = individual_round(200, 1, RoundStatus::Completed);
    round.scores = vec![
        score(&round, 1, 0, 4),
        score(&round, 1, 1, 4),
        score(&round, 2, 0, 4),
        score(&round, 2, 1, 4),
    ];
    let result = build_tournament_leaderboard(
        &TournamentLeaderboardFacts {
            tournament_id: id(100),
            participants: vec![
                participant(2, "bob"),
                participant(1, "Ada"),
                participant(3, "Cara"),
            ],
            rounds: vec![round],
        },
        LeaderboardMetric::Gross,
    )
    .unwrap();
    assert_eq!(result.entries[0].position, Some(1));
    assert_eq!(result.entries[1].position, Some(1));
    assert!(result.entries[0].tied && result.entries[1].tied);
    assert_eq!(result.entries[2].position, None);
}

#[test]
fn completed_rounds_and_confirmations_require_complete_cards() {
    let mut completed = individual_round(200, 1, RoundStatus::Completed);
    completed.scores = vec![score(&completed, 1, 0, 4)];
    assert_eq!(
        build_round_leaderboard(&completed, LeaderboardMetric::Gross),
        Err(LeaderboardError::InvalidStoredData)
    );

    let mut confirmed_partial = individual_round(201, 1, RoundStatus::Open);
    confirmed_partial.scores = vec![score(&confirmed_partial, 1, 0, 4)];
    confirmed_partial.confirmations = vec![ConfirmationFact {
        round_id: confirmed_partial.round.round_id,
        player_id: Some(id(1)),
        team_id: None,
    }];
    assert_eq!(
        build_round_leaderboard(&confirmed_partial, LeaderboardMetric::Gross),
        Err(LeaderboardError::InvalidStoredData)
    );
}

#[test]
fn case_insensitive_name_ties_fall_directly_to_uuid() {
    let mut individual = individual_round(200, 1, RoundStatus::Open);
    individual.snapshots[0].display_name = "zed".to_owned();
    individual.snapshots[1].display_name = "ZED".to_owned();
    let round = build_round_leaderboard(&individual, LeaderboardMetric::Gross).unwrap();
    assert_eq!(round.entries[0].owner.id(), id(1));

    let tournament = build_tournament_leaderboard(
        &TournamentLeaderboardFacts {
            tournament_id: id(100),
            participants: vec![participant(2, "ZED"), participant(1, "zed")],
            rounds: Vec::new(),
        },
        LeaderboardMetric::Gross,
    )
    .unwrap();
    assert_eq!(tournament.entries[0].player_id, id(1));

    let team_id = id(310);
    let scramble_round = round_fact(300, 1, RoundStatus::Open, ScoringFormat::TeamScramble);
    let scramble = RoundLeaderboardFacts {
        holes: holes(scramble_round.round_id),
        snapshots: vec![
            snapshot(scramble_round.round_id, 1, "zed", 0, 0),
            snapshot(scramble_round.round_id, 2, "ZED", 0, 0),
        ],
        team_snapshots: Vec::new(),
        teams: vec![TeamFact {
            round_id: scramble_round.round_id,
            team_id,
            team_name: "Pair".to_owned(),
        }],
        memberships: vec![
            member(scramble_round.round_id, team_id, 2, "ZED"),
            member(scramble_round.round_id, team_id, 1, "zed"),
        ],
        scores: Vec::new(),
        confirmations: Vec::new(),
        round: scramble_round,
    };
    let result = build_round_leaderboard(&scramble, LeaderboardMetric::Gross).unwrap();
    assert_eq!(result.entries[0].members[0].player_id, id(1));
}

#[test]
fn stored_fact_validation_rejects_missing_snapshots_duplicate_attribution_and_invalid_holes() {
    let team_id = id(310);
    let round = round_fact(300, 1, RoundStatus::Open, ScoringFormat::TeamScramble);
    let base = RoundLeaderboardFacts {
        holes: holes(round.round_id),
        snapshots: vec![
            snapshot(round.round_id, 1, "Ada", 8, 8),
            snapshot(round.round_id, 2, "Bob", 20, 20),
        ],
        team_snapshots: Vec::new(),
        teams: vec![TeamFact {
            round_id: round.round_id,
            team_id,
            team_name: "Pair".to_owned(),
        }],
        memberships: vec![
            member(round.round_id, team_id, 1, "Ada"),
            member(round.round_id, team_id, 2, "Bob"),
        ],
        scores: Vec::new(),
        confirmations: Vec::new(),
        round,
    };

    let mut missing_snapshot = base.clone();
    missing_snapshot.snapshots.pop();
    assert_eq!(
        build_round_leaderboard(&missing_snapshot, LeaderboardMetric::Net),
        Err(LeaderboardError::InvalidStoredData)
    );

    let mut duplicate_attribution = base.clone();
    duplicate_attribution.teams.push(TeamFact {
        round_id: duplicate_attribution.round.round_id,
        team_id: id(311),
        team_name: "Other".to_owned(),
    });
    duplicate_attribution.memberships.push(member(
        duplicate_attribution.round.round_id,
        id(311),
        1,
        "Ada",
    ));
    assert_eq!(
        build_round_leaderboard(&duplicate_attribution, LeaderboardMetric::Net),
        Err(LeaderboardError::InvalidStoredData)
    );

    let mut invalid_holes = individual_round(200, 1, RoundStatus::Open);
    invalid_holes.holes[1].stroke_index = 1;
    assert_eq!(
        build_round_leaderboard(&invalid_holes, LeaderboardMetric::Net),
        Err(LeaderboardError::InvalidStoredData)
    );
}
