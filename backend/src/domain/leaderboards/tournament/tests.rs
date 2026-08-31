use uuid::Uuid;

use crate::domain::leaderboards::{
    HoleFact, LeaderboardError, LeaderboardMetric, LeaderboardOwner, MembershipFact,
    ParticipantFact, RoundFact, RoundLeaderboardFacts, ScoreFact, SnapshotFact, TeamFact,
    TeamSnapshotFact, TournamentLeaderboardFacts, build_tournament_leaderboard,
};
use crate::domain::models::{ParticipantStatus, RoundStatus, ScoringFormat};

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn completed_round(round_number: i16, scores: &[(u128, i16)]) -> RoundLeaderboardFacts {
    let round_id = id(100 + round_number as u128);
    let holes = vec![
        HoleFact {
            round_id,
            hole_id: id(1_000 + round_number as u128 * 10),
            hole_number: 1,
            par: 4,
            stroke_index: 1,
        },
        HoleFact {
            round_id,
            hole_id: id(1_001 + round_number as u128 * 10),
            hole_number: 2,
            par: 4,
            stroke_index: 2,
        },
    ];
    RoundLeaderboardFacts {
        round: RoundFact {
            round_id,
            tournament_id: id(1),
            round_number,
            status: RoundStatus::Completed,
            scoring_format: ScoringFormat::IndividualStrokePlay,
            number_of_holes: 2,
            handicap_enabled: true,
            handicap_allowance_percent: 100,
        },
        snapshots: scores
            .iter()
            .map(|(player, _)| SnapshotFact {
                round_id,
                player_id: id(*player),
                display_name: format!("Player {player}"),
                course_handicap: if *player == 1 {
                    match round_number {
                        1 => 0,
                        2 => 4,
                        _ => 12,
                    }
                } else {
                    0
                },
                playing_handicap: if *player == 1 {
                    match round_number {
                        1 => 0,
                        2 => 4,
                        _ => 12,
                    }
                } else {
                    0
                },
            })
            .collect(),
        scores: scores
            .iter()
            .flat_map(|(player, gross)| {
                holes.iter().map(move |hole| ScoreFact {
                    round_id,
                    hole_id: hole.hole_id,
                    player_id: Some(id(*player)),
                    team_id: None,
                    gross_strokes: *gross,
                })
            })
            .collect(),
        holes,
        team_snapshots: Vec::new(),
        teams: Vec::new(),
        memberships: Vec::new(),
        confirmations: Vec::new(),
    }
}

fn completed_foursomes_round(round_number: i16) -> RoundLeaderboardFacts {
    let round_id = id(200 + round_number as u128);
    let team_id = id(500 + round_number as u128);
    let holes = vec![
        HoleFact {
            round_id,
            hole_id: id(2_000 + round_number as u128 * 10),
            hole_number: 1,
            par: 4,
            stroke_index: 1,
        },
        HoleFact {
            round_id,
            hole_id: id(2_001 + round_number as u128 * 10),
            hole_number: 2,
            par: 4,
            stroke_index: 2,
        },
    ];
    RoundLeaderboardFacts {
        round: RoundFact {
            round_id,
            tournament_id: id(1),
            round_number,
            status: RoundStatus::Locked,
            scoring_format: ScoringFormat::TwoPlayerFoursomes,
            number_of_holes: 2,
            handicap_enabled: true,
            handicap_allowance_percent: 50,
        },
        snapshots: vec![
            SnapshotFact {
                round_id,
                player_id: id(1),
                display_name: "Ada".to_owned(),
                course_handicap: 2,
                playing_handicap: 2,
            },
            SnapshotFact {
                round_id,
                player_id: id(2),
                display_name: "Bob".to_owned(),
                course_handicap: 2,
                playing_handicap: 2,
            },
        ],
        team_snapshots: vec![TeamSnapshotFact {
            round_id,
            team_id,
            playing_handicap: 2,
        }],
        teams: vec![TeamFact {
            round_id,
            team_id,
            team_name: "Frozen pair".to_owned(),
        }],
        memberships: vec![
            MembershipFact {
                round_id,
                team_id,
                player_id: id(1),
                display_name: "Ada".to_owned(),
                display_order: Some(1),
            },
            MembershipFact {
                round_id,
                team_id,
                player_id: id(2),
                display_name: "Bob".to_owned(),
                display_order: Some(2),
            },
        ],
        scores: holes
            .iter()
            .map(|hole| ScoreFact {
                round_id,
                hole_id: hole.hole_id,
                player_id: None,
                team_id: Some(team_id),
                gross_strokes: 5,
            })
            .collect(),
        holes,
        confirmations: Vec::new(),
    }
}

fn facts(rounds: Vec<RoundLeaderboardFacts>, counted_rounds: usize) -> TournamentLeaderboardFacts {
    TournamentLeaderboardFacts {
        tournament_id: id(1),
        counted_rounds,
        participants: vec![
            ParticipantFact {
                player_id: id(1),
                display_name: "Ada".to_owned(),
                status: ParticipantStatus::Active,
            },
            ParticipantFact {
                player_id: id(2),
                display_name: "Bob".to_owned(),
                status: ParticipantStatus::Active,
            },
            ParticipantFact {
                player_id: id(3),
                display_name: "Cara".to_owned(),
                status: ParticipantStatus::Active,
            },
        ],
        rounds,
    }
}

#[test]
fn gross_and_net_select_independently_and_return_round_order() {
    let input = facts(
        vec![
            completed_round(3, &[(1, 5)]),
            completed_round(1, &[(1, 4)]),
            completed_round(2, &[(1, 3)]),
        ],
        2,
    );
    let gross = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    let net = build_tournament_leaderboard(&input, LeaderboardMetric::Net).unwrap();
    let gross_entry = &gross.entries[0];
    let net_entry = &net.entries[0];
    assert_eq!(gross.required_counted_rounds, 2);
    assert_eq!(gross_entry.completed_rounds, 3);
    assert_eq!(gross_entry.counted_contributions, 2);
    assert!(gross_entry.eligible);
    assert_eq!(gross_entry.score_to_par, -2);
    assert_eq!(net_entry.score_to_par, -16);
    assert_eq!(
        gross_entry
            .contributions
            .iter()
            .map(|item| (item.round_id, item.counted))
            .collect::<Vec<_>>(),
        vec![(id(101), true), (id(102), true), (id(103), false)]
    );
    assert_eq!(
        net_entry
            .contributions
            .iter()
            .map(|item| (item.round_id, item.counted))
            .collect::<Vec<_>>(),
        vec![(id(101), false), (id(102), true), (id(103), true)]
    );
}

#[test]
fn ranks_by_count_then_score_and_keeps_sporting_ties() {
    let input = facts(
        vec![
            completed_round(1, &[(1, 4), (2, 4), (3, 3)]),
            completed_round(2, &[(1, 5), (2, 5)]),
        ],
        2,
    );
    let result = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    assert_eq!(result.entries[0].player_id, id(1));
    assert_eq!(result.entries[0].position, Some(1));
    assert_eq!(result.entries[1].player_id, id(2));
    assert_eq!(result.entries[1].position, Some(1));
    assert!(result.entries[0].tied && result.entries[1].tied);
    assert_eq!(result.entries[2].player_id, id(3));
    assert_eq!(result.entries[2].position, Some(3));
    assert!(!result.entries[2].eligible);
}

#[test]
fn cutoff_ties_select_by_round_order() {
    let input = facts(
        vec![
            completed_round(3, &[(2, 4)]),
            completed_round(1, &[(2, 4)]),
            completed_round(2, &[(2, 4)]),
        ],
        2,
    );
    let result = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    let entry = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(2))
        .unwrap();
    assert_eq!(
        entry
            .contributions
            .iter()
            .map(|item| (item.round_id, item.counted))
            .collect::<Vec<_>>(),
        vec![(id(101), true), (id(102), true), (id(103), false)]
    );
}

#[test]
fn corrupt_open_round_facts_fail_closed_without_contributing() {
    let mut open = completed_round(1, &[(1, 4)]);
    open.round.status = RoundStatus::Open;
    open.holes[1].stroke_index = 1;
    assert_eq!(
        build_tournament_leaderboard(&facts(vec![open], 1), LeaderboardMetric::Gross),
        Err(LeaderboardError::InvalidStoredData)
    );
}

#[test]
fn duplicate_player_round_attribution_is_rejected() {
    let mut round = completed_foursomes_round(1);
    round.memberships[1].player_id = id(1);
    round.memberships[1].display_name = "Ada".to_owned();
    assert_eq!(
        build_tournament_leaderboard(&facts(vec![round], 1), LeaderboardMetric::Gross),
        Err(LeaderboardError::InvalidStoredData)
    );
}

#[test]
fn foursomes_contribution_preserves_tagged_team_owner_for_each_frozen_member() {
    let result = build_tournament_leaderboard(
        &facts(vec![completed_foursomes_round(1)], 1),
        LeaderboardMetric::Net,
    )
    .unwrap();
    for player_id in [id(1), id(2)] {
        let entry = result
            .entries
            .iter()
            .find(|entry| entry.player_id == player_id)
            .unwrap();
        assert_eq!(entry.completed_rounds, 1);
        assert_eq!(entry.contributions.len(), 1);
        let contribution = &entry.contributions[0];
        assert_eq!(contribution.round_id, id(201));
        assert_eq!(contribution.owner, LeaderboardOwner::Team { id: id(501) });
        assert_eq!(contribution.owner_name, "Frozen pair");
        assert_eq!((contribution.gross_total, contribution.net_total), (10, 8));
        assert!(contribution.counted);
    }
}

#[test]
fn zero_results_are_unranked_and_invalid_required_count_is_rejected() {
    let result =
        build_tournament_leaderboard(&facts(Vec::new(), 1), LeaderboardMetric::Gross).unwrap();
    assert!(result.entries.iter().all(|entry| entry.position.is_none()));
    assert_eq!(
        build_tournament_leaderboard(&facts(Vec::new(), 0), LeaderboardMetric::Gross),
        Err(LeaderboardError::InvalidStoredData)
    );
}
