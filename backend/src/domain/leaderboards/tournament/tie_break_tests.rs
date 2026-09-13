use super::*;
use crate::domain::models::TournamentTieBreakPolicy;

fn configured(rounds: Vec<RoundLeaderboardFacts>, required: usize) -> TournamentLeaderboardFacts {
    let mut input = facts(rounds, required);
    input.final_round_number = 3;
    input.tie_break_policy = TournamentTieBreakPolicy::FinalRoundScore;
    input
}

fn round(number: i16, holes: i16, values: &[(u128, i16)]) -> RoundLeaderboardFacts {
    let mut result = completed_round(number, values);
    result.round.number_of_holes = holes;
    result.holes = (1..=holes)
        .map(|hole| HoleFact {
            round_id: result.round.round_id,
            hole_id: id(10_000 + number as u128 * 100 + hole as u128),
            hole_number: hole,
            par: 4,
            stroke_index: hole,
        })
        .collect();
    for snapshot in &mut result.snapshots {
        snapshot.course_handicap = 0;
        snapshot.playing_handicap = 0;
    }
    result.scores = values
        .iter()
        .flat_map(|(player, gross)| {
            result.holes.iter().map(move |hole| ScoreFact {
                round_id: hole.round_id,
                hole_id: hole.hole_id,
                player_id: Some(id(*player)),
                team_id: None,
                gross_strokes: *gross,
            })
        })
        .collect();
    result
}

#[test]
fn final_excluded_from_best_n_resolves_with_residual_competition_positions_for_nine_and_eighteen() {
    for holes in [9, 18] {
        let input = configured(
            vec![
                round(1, holes, &[(1, 4), (2, 4), (3, 4)]),
                round(3, holes, &[(1, 6), (2, 5), (3, 5)]),
            ],
            1,
        );
        let board = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
        assert_eq!(
            board
                .entries
                .iter()
                .map(|e| (e.player_id, e.position, e.tied, e.tie_break_score_to_par))
                .collect::<Vec<_>>(),
            vec![
                (id(2), Some(1), true, Some(i32::from(holes))),
                (id(3), Some(1), true, Some(i32::from(holes))),
                (id(1), Some(3), false, Some(i32::from(holes) * 2))
            ]
        );
        assert!(board.entries.iter().all(|e| {
            e.contributions
                .iter()
                .find(|c| c.round_id == id(103))
                .is_some_and(|c| !c.counted)
        }));
        let mut shared = input.clone();
        shared.tie_break_policy = TournamentTieBreakPolicy::SharedPositions;
        let board = build_tournament_leaderboard(&shared, LeaderboardMetric::Gross).unwrap();
        assert!(
            board
                .entries
                .iter()
                .all(|e| e.position == Some(1) && e.tied && e.tie_break_score_to_par.is_none())
        );
    }
}

#[test]
fn gross_and_net_compare_only_the_requested_metric() {
    let mut final_round = round(3, 9, &[(1, 6), (2, 5), (3, 5)]);
    final_round.snapshots[0].playing_handicap = 12;
    let input = configured(vec![round(1, 9, &[(1, 4), (2, 4), (3, 4)]), final_round], 1);
    let gross = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    let net = build_tournament_leaderboard(&input, LeaderboardMetric::Net).unwrap();
    assert_eq!(
        (
            gross.entries[0].player_id,
            gross.entries[0].tie_break_score_to_par
        ),
        (id(2), Some(9))
    );
    assert_eq!(
        (
            net.entries[0].player_id,
            net.entries[0].tie_break_score_to_par
        ),
        (id(1), Some(6))
    );
    assert!(net.entries.iter().all(|e| e.score_to_par == 0));
}

#[test]
fn missing_one_final_keeps_the_entire_group_tied_and_scheduled_final_is_not_loaded_maximum() {
    for finals in [vec![(1, 6), (2, 5)], vec![]] {
        let mut rounds = vec![
            round(1, 9, &[(1, 4), (2, 4), (3, 4)]),
            round(2, 9, &[(1, 6), (2, 5), (3, 7)]),
        ];
        if !finals.is_empty() {
            rounds.push(round(3, 9, &finals));
        }
        let board =
            build_tournament_leaderboard(&configured(rounds, 1), LeaderboardMetric::Gross).unwrap();
        assert!(
            board
                .entries
                .iter()
                .all(|e| e.position == Some(1) && e.tied && e.tie_break_score_to_par.is_none())
        );
    }
}

#[test]
fn provisional_complete_or_unstarted_final_cannot_resolve() {
    for scored in [true, false] {
        let mut final_round = round(3, 9, &[(1, 5), (2, 6), (3, 7)]);
        final_round.round.status = RoundStatus::Open;
        if !scored {
            final_round.scores.clear();
        }
        let input = configured(vec![round(1, 9, &[(1, 4), (2, 4), (3, 4)]), final_round], 1);
        let board = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
        assert!(
            board
                .entries
                .iter()
                .all(|e| e.position == Some(1) && e.tied && e.tie_break_score_to_par.is_none())
        );
    }
}

#[test]
fn selected_provisional_blocks_otherwise_comparable_group() {
    let mut open = round(2, 9, &[(1, 4), (2, 4), (3, 4)]);
    open.round.status = RoundStatus::Open;
    let input = configured(
        vec![
            round(1, 9, &[(1, 5), (2, 5), (3, 5)]),
            open,
            round(3, 9, &[(1, 6), (2, 7), (3, 8)]),
        ],
        2,
    );
    let board = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    assert!(board.entries.iter().all(|e| e.eligible
        && e.position == Some(1)
        && e.tied
        && e.tie_break_score_to_par.is_none()));
}

#[test]
fn mandatory_qualification_and_primary_totals_stay_authoritative() {
    let mut input = configured(
        vec![
            round(1, 9, &[(1, 4), (2, 4), (3, 4)]),
            round(3, 9, &[(1, 6), (2, 5), (3, 7)]),
        ],
        3,
    );
    input.mandatory_round_id = Some(id(102));
    let board = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    assert!(
        board
            .entries
            .iter()
            .all(|e| !e.eligible && e.tie_break_score_to_par.is_none())
    );
    input.counted_rounds = 1;
    let board = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    assert!(
        board
            .entries
            .iter()
            .all(|e| e.position.is_none() && !e.tied && e.tie_break_score_to_par.is_none())
    );
}

#[test]
fn hidden_final_contribution_never_changes_rank_or_metadata() {
    let input = configured(
        vec![
            round(1, 18, &[(1, 4), (2, 4), (3, 4)]),
            round(3, 18, &[(1, 6), (2, 5), (3, 7)]),
        ],
        1,
    );
    let projection = VisibilityMetadata {
        mode: VisibilityMode::FrontNine,
    };
    let board = build_tournament_leaderboard_projected(
        &input,
        LeaderboardMetric::Gross,
        projection,
        Some(id(103)),
        projection,
    )
    .unwrap();
    assert!(board.entries.iter().all(|e| e.position == Some(1)
        && e.tied
        && e.tie_break_score_to_par.is_none()
        && e.contributions.len() == 1));
    assert!(
        build_tournament_leaderboard(&input, LeaderboardMetric::Gross)
            .unwrap()
            .entries
            .iter()
            .all(|e| e.tie_break_score_to_par.is_some())
    );
}

#[test]
fn final_teammates_share_preserved_contribution_despite_earlier_different_owners() {
    let mut input = configured(
        vec![
            completed_round(1, &[(1, 4), (2, 4)]),
            completed_foursomes_round(3),
        ],
        1,
    );
    input.participants.retain(|p| p.player_id != id(3));
    let board = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    assert!(
        board
            .entries
            .iter()
            .all(|e| e.position == Some(1) && e.tied && e.tie_break_score_to_par == Some(2))
    );
    assert_eq!(
        board.entries[0].contributions[1].owner,
        board.entries[1].contributions[1].owner
    );
    assert_ne!(
        board.entries[0].contributions[0].owner,
        board.entries[1].contributions[0].owner
    );
}

#[test]
fn untied_primary_scores_do_not_expose_unused_comparison_values() {
    let input = configured(
        vec![
            round(1, 9, &[(1, 3), (2, 4), (3, 5)]),
            round(3, 9, &[(1, 8), (2, 7), (3, 6)]),
        ],
        1,
    );
    let board = build_tournament_leaderboard(&input, LeaderboardMetric::Gross).unwrap();
    assert_eq!(
        board
            .entries
            .iter()
            .map(|entry| entry.player_id)
            .collect::<Vec<_>>(),
        vec![id(1), id(2), id(3)]
    );
    assert!(
        board
            .entries
            .iter()
            .all(|entry| !entry.tied && entry.tie_break_score_to_par.is_none())
    );
}
