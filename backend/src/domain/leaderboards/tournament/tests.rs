use uuid::Uuid;

use crate::domain::leaderboards::{
    HoleFact, LeaderboardError, LeaderboardMetric, LeaderboardOwner, MembershipFact,
    ParticipantFact, RoundFact, RoundLeaderboardFacts, ScoreFact, SnapshotFact, TeamFact,
    TeamSnapshotFact, TournamentLeaderboardFacts, build_tournament_leaderboard,
    build_tournament_leaderboard_projected,
};
use crate::domain::models::{ParticipantStatus, RoundStatus, ScoringFormat};
use crate::domain::score_visibility::{VisibilityMetadata, VisibilityMode};

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
        mandatory_round_id: None,
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

fn open_individual_round(
    round_number: i16,
    scores: &[(u128, i16)],
    retained_scores: &[(u128, usize)],
) -> RoundLeaderboardFacts {
    let mut round = completed_round(round_number, scores);
    round.round.status = RoundStatus::Open;
    let holes = round.holes.clone();
    round.scores.retain(|score| {
        retained_scores.iter().any(|(player_id, count)| {
            score.player_id == Some(id(*player_id))
                && holes
                    .iter()
                    .position(|hole| hole.hole_id == score.hole_id)
                    .is_some_and(|index| index < *count)
        })
    });
    round
}

fn final_open_round(player_id: u128) -> RoundLeaderboardFacts {
    let round_id = id(118);
    let holes = (1..=18)
        .map(|hole_number| HoleFact {
            round_id,
            hole_id: id(10_000 + hole_number as u128),
            hole_number,
            par: 4,
            stroke_index: hole_number,
        })
        .collect::<Vec<_>>();
    RoundLeaderboardFacts {
        round: RoundFact {
            round_id,
            tournament_id: id(1),
            round_number: 18,
            status: RoundStatus::Open,
            scoring_format: ScoringFormat::IndividualStrokePlay,
            number_of_holes: 18,
            handicap_enabled: true,
            handicap_allowance_percent: 100,
        },
        snapshots: vec![SnapshotFact {
            round_id,
            player_id: id(player_id),
            display_name: format!("Player {player_id}"),
            course_handicap: 0,
            playing_handicap: 0,
        }],
        scores: holes
            .iter()
            .map(|hole| ScoreFact {
                round_id,
                hole_id: hole.hole_id,
                player_id: Some(id(player_id)),
                team_id: None,
                gross_strokes: if hole.hole_number <= 9 { 4 } else { 2 },
            })
            .collect(),
        holes,
        team_snapshots: Vec::new(),
        teams: Vec::new(),
        memberships: Vec::new(),
        confirmations: Vec::new(),
    }
}

fn visibility(mode: VisibilityMode) -> VisibilityMetadata {
    VisibilityMetadata { mode }
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
fn mandatory_round_reserves_a_slot_for_each_metric() {
    let mut input = facts(
        vec![
            completed_round(1, &[(1, 4)]),
            completed_round(2, &[(1, 3)]),
            completed_round(3, &[(1, 5)]),
        ],
        2,
    );
    input.mandatory_round_id = Some(id(101));

    let gross = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    let net = build_tournament_leaderboard(&input, LeaderboardMetric::Net).unwrap();
    for result in [&gross, &net] {
        assert_eq!(result.mandatory_round_id, Some(id(101)));
        let entry = &result.entries[0];
        assert_eq!(entry.counted_contributions, 2);
        assert!(entry.eligible);
        let mandatory = entry
            .contributions
            .iter()
            .find(|contribution| contribution.round_id == id(101))
            .unwrap();
        assert!(mandatory.mandatory && mandatory.counted);
    }
    assert_ne!(
        gross.entries[0]
            .contributions
            .iter()
            .find(|contribution| !contribution.mandatory && contribution.counted)
            .map(|contribution| contribution.round_id),
        net.entries[0]
            .contributions
            .iter()
            .find(|contribution| !contribution.mandatory && contribution.counted)
            .map(|contribution| contribution.round_id)
    );
}

#[test]
fn missing_mandatory_round_reserves_slot_and_n_one_counts_only_mandatory() {
    let mut missing = facts(
        vec![completed_round(1, &[(1, 4)]), completed_round(2, &[(1, 3)])],
        2,
    );
    missing.mandatory_round_id = Some(id(103));
    let result = build_tournament_leaderboard(&missing, LeaderboardMetric::Gross).unwrap();
    assert_eq!(result.entries[0].counted_contributions, 1);
    assert!(!result.entries[0].eligible);

    let mut only_mandatory = facts(
        vec![completed_round(1, &[(1, 3)]), completed_round(2, &[(1, 5)])],
        1,
    );
    only_mandatory.mandatory_round_id = Some(id(102));
    let result = build_tournament_leaderboard(&only_mandatory, LeaderboardMetric::Gross).unwrap();
    let counted = result.entries[0]
        .contributions
        .iter()
        .filter(|contribution| contribution.counted)
        .collect::<Vec<_>>();
    assert_eq!(counted.len(), 1);
    assert_eq!(counted[0].round_id, id(102));
    assert!(counted[0].mandatory);
}

#[test]
fn n_one_missing_mandatory_with_optional_result_is_unranked() {
    let mut input = facts(vec![completed_round(1, &[(1, 3)])], 1);
    input.mandatory_round_id = Some(id(102));

    let result = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    let entry = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(1))
        .unwrap();
    assert_eq!(entry.completed_rounds, 1);
    assert_eq!(entry.counted_contributions, 0);
    assert!(!entry.eligible);
    assert_eq!(entry.position, None);
    assert!(!entry.tied);
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
    let mut input = facts(vec![completed_foursomes_round(1)], 1);
    input.mandatory_round_id = Some(id(201));
    let result = build_tournament_leaderboard(&input, LeaderboardMetric::Net).unwrap();
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
        assert!(contribution.mandatory);
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

#[test]
fn only_highest_open_round_contributes_and_unstarted_owners_stay_unranked() {
    let older = open_individual_round(2, &[(1, 2)], &[(1, 1)]);
    let latest = open_individual_round(3, &[(1, 4), (2, 5)], &[(1, 2)]);
    let result =
        build_tournament_leaderboard(&facts(vec![older, latest], 2), LeaderboardMetric::Gross)
            .unwrap();

    assert_eq!(result.current_round_id, Some(id(103)));
    let ada = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(1))
        .unwrap();
    assert_eq!(ada.completed_rounds, 0);
    assert_eq!(ada.counted_contributions, 0);
    assert!(!ada.eligible);
    assert_eq!(ada.position, Some(1));
    assert_eq!(ada.contributions.len(), 1);
    assert_eq!(ada.contributions[0].round_id, id(103));
    assert!(ada.contributions[0].provisional && ada.contributions[0].counted);
    assert_eq!(
        (
            ada.contributions[0].holes_scored,
            ada.contributions[0].number_of_holes
        ),
        (2, 2)
    );
    let bob = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(2))
        .unwrap();
    assert!(bob.contributions.is_empty());
    assert_eq!(bob.position, None);
}

#[test]
fn provisional_can_displace_displayed_best_n_without_granting_qualification() {
    let completed = completed_round(1, &[(1, 5), (2, 3)]);
    let open = open_individual_round(2, &[(1, 2), (2, 4)], &[(1, 1), (2, 1)]);
    let result =
        build_tournament_leaderboard(&facts(vec![completed, open], 1), LeaderboardMetric::Gross)
            .unwrap();

    let ada = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(1))
        .unwrap();
    assert_eq!((ada.completed_rounds, ada.counted_contributions), (1, 1));
    assert!(ada.eligible);
    assert_eq!(
        ada.contributions
            .iter()
            .map(|item| (item.round_id, item.provisional, item.counted))
            .collect::<Vec<_>>(),
        vec![(id(101), false, false), (id(102), true, true)]
    );
    assert_eq!(
        (ada.gross_total, ada.par_total, ada.score_to_par),
        (2, 4, -2)
    );
}

#[test]
fn completed_qualification_precedes_score_and_progress_only_orders_sporting_ties() {
    let completed = completed_round(1, &[(1, 6)]);
    let mut open = open_individual_round(2, &[(1, 5), (2, 3), (3, 3)], &[(1, 1), (2, 1), (3, 2)]);
    open.scores
        .iter_mut()
        .filter(|score| score.player_id == Some(id(3)))
        .max_by_key(|score| score.hole_id)
        .unwrap()
        .gross_strokes = 4;
    let result =
        build_tournament_leaderboard(&facts(vec![completed, open], 2), LeaderboardMetric::Gross)
            .unwrap();

    assert_eq!(result.entries[0].player_id, id(1));
    assert_eq!(result.entries[0].counted_contributions, 1);
    let bob = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(2))
        .unwrap();
    let cara = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(3))
        .unwrap();
    assert_eq!((bob.position, cara.position), (Some(2), Some(2)));
    assert!(bob.tied && cara.tied);
    assert!(
        result
            .entries
            .iter()
            .position(|entry| entry.player_id == id(3))
            < result
                .entries
                .iter()
                .position(|entry| entry.player_id == id(2))
    );
}

#[test]
fn scored_open_mandatory_occupies_reserved_display_slot_but_not_eligibility() {
    let completed = completed_round(1, &[(1, 3)]);
    let open = open_individual_round(2, &[(1, 4)], &[(1, 1)]);
    let mut input = facts(vec![completed, open], 2);
    input.mandatory_round_id = Some(id(102));
    let result = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    let entry = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(1))
        .unwrap();

    assert_eq!(
        (entry.completed_rounds, entry.counted_contributions),
        (1, 1)
    );
    assert!(!entry.eligible);
    assert_eq!(
        entry
            .contributions
            .iter()
            .filter(|item| item.counted)
            .count(),
        2
    );
    let mandatory = entry
        .contributions
        .iter()
        .find(|item| item.mandatory)
        .unwrap();
    assert!(mandatory.provisional && mandatory.counted);
}

#[test]
fn open_team_card_is_attributed_once_to_each_frozen_member() {
    let mut open = completed_foursomes_round(2);
    open.round.status = RoundStatus::Open;
    open.scores.truncate(1);
    let result =
        build_tournament_leaderboard(&facts(vec![open], 1), LeaderboardMetric::Net).unwrap();

    for player_id in [id(1), id(2)] {
        let entry = result
            .entries
            .iter()
            .find(|entry| entry.player_id == player_id)
            .unwrap();
        assert_eq!(entry.contributions.len(), 1);
        let contribution = &entry.contributions[0];
        assert!(contribution.provisional);
        assert_eq!(contribution.owner, LeaderboardOwner::Team { id: id(502) });
        assert_eq!(
            (contribution.holes_scored, contribution.number_of_holes),
            (1, 2)
        );
    }
}

#[test]
fn final_open_projection_never_derives_back_nine_totals_or_progress() {
    let input = facts(vec![final_open_round(1)], 1);
    let restricted = build_tournament_leaderboard_projected(
        &input,
        LeaderboardMetric::Gross,
        visibility(VisibilityMode::FrontNine),
        None,
        visibility(VisibilityMode::FrontNine),
    )
    .unwrap();
    let admin = build_tournament_leaderboard_projected(
        &input,
        LeaderboardMetric::Gross,
        visibility(VisibilityMode::Full),
        None,
        visibility(VisibilityMode::Full),
    )
    .unwrap();

    let restricted_contribution = &restricted.entries[0].contributions[0];
    assert_eq!(
        (
            restricted_contribution.holes_scored,
            restricted_contribution.number_of_holes,
            restricted_contribution.gross_total,
            restricted_contribution.par_total,
            restricted_contribution.score_to_par,
        ),
        (9, 18, 36, 36, 0)
    );
    let admin_contribution = &admin.entries[0].contributions[0];
    assert_eq!(
        (
            admin_contribution.holes_scored,
            admin_contribution.gross_total
        ),
        (18, 54)
    );
}

#[test]
fn back_nine_only_scoring_creates_no_restricted_provisional_identity() {
    let mut final_round = final_open_round(1);
    final_round.scores.retain(|score| {
        final_round
            .holes
            .iter()
            .find(|hole| hole.hole_id == score.hole_id)
            .is_some_and(|hole| hole.hole_number > 9)
    });
    let result = build_tournament_leaderboard_projected(
        &facts(vec![final_round], 1),
        LeaderboardMetric::Gross,
        visibility(VisibilityMode::FrontNine),
        None,
        visibility(VisibilityMode::FrontNine),
    )
    .unwrap();

    let entry = result
        .entries
        .iter()
        .find(|entry| entry.player_id == id(1))
        .unwrap();
    assert!(entry.contributions.is_empty());
    assert_eq!(entry.position, None);
}

#[test]
fn hidden_final_metadata_does_not_redact_a_different_current_round() {
    let open = open_individual_round(3, &[(1, 4)], &[(1, 2)]);
    let result = build_tournament_leaderboard_projected(
        &facts(vec![open], 1),
        LeaderboardMetric::Gross,
        visibility(VisibilityMode::FrontNine),
        None,
        visibility(VisibilityMode::Full),
    )
    .unwrap();

    assert_eq!(result.visibility.mode, VisibilityMode::FrontNine);
    assert_eq!(result.entries[0].contributions[0].holes_scored, 2);
}
