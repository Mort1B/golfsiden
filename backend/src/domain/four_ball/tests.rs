use super::*;
use crate::domain::{handicap, models::ScoringFormat, player_score_input::InvalidGrossScore};

const A: Uuid = Uuid::from_u128(1);
const B: Uuid = Uuid::from_u128(2);
const INDEXES: [u8; 18] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
];

fn numeric(value: i32) -> PlayerHoleInput {
    PlayerHoleInput::Numeric(GrossScore::new(value).unwrap())
}

fn player(player_id: Uuid, playing_handicap: i16, input: PlayerHoleInput) -> PlayerHole {
    PlayerHole {
        player_id,
        playing_handicap,
        input,
    }
}

fn partner(player_id: Uuid, playing_handicap: i16, input: PlayerHoleInput) -> PartnerCard {
    PartnerCard {
        player_id,
        playing_handicap,
        holes: [input; 18],
    }
}

#[test]
fn allowance_uses_uncapped_unrounded_course_handicap() {
    for (index, expected_course, expected_playing) in [(96, 10, 8), (200, 20, 17), (-100, -10, -8)]
    {
        let numerator = handicap::course_handicap_numerator(index, 113, 720, 72);
        let result = calculate_handicap(numerator, DEFAULT_ALLOWANCE_PERCENT, true).unwrap();
        assert_eq!(result.course_handicap, expected_course);
        assert_eq!(result.playing_handicap, expected_playing);
    }
    let numerator = handicap::course_handicap_numerator(540, 155, 722, 72);
    let result = calculate_handicap(numerator, 85, true).unwrap();
    assert_eq!(result.course_handicap, 74);
    assert_eq!(result.playing_handicap, 63);
}

#[test]
fn signed_half_rounding_does_not_change_legacy_policy() {
    for (numerator, expected) in [
        (564, 0),
        (565, 1),
        (566, 1),
        (-564, 0),
        (-565, 0),
        (-566, -1),
    ] {
        let result = calculate_handicap(numerator, 100, true).unwrap();
        assert_eq!(result.course_handicap, expected);
        assert_eq!(result.playing_handicap, expected);
    }
    let legacy = handicap::calculate(
        0,
        113,
        715,
        72,
        100,
        true,
        ScoringFormat::IndividualStrokePlay,
    );
    assert_eq!(legacy.course_handicap, -1);
    assert_eq!(legacy.playing_handicap, -1);
}

#[test]
fn allowances_and_snapshot_range_are_checked_without_panics() {
    for allowance in [-1, 101, i16::MIN, i16::MAX] {
        assert_eq!(
            calculate_handicap(0, allowance, true),
            Err(FourBallError::InvalidAllowance)
        );
        assert_eq!(
            calculate_handicap(0, allowance, false),
            Err(FourBallError::InvalidAllowance)
        );
    }
    for numerator in [
        i64::MIN,
        i64::MAX,
        i64::from(i16::MAX) * 1130 + 565,
        i64::from(i16::MIN) * 1130 - 566,
    ] {
        assert_eq!(
            calculate_handicap(numerator, 85, true),
            Err(FourBallError::HandicapOutOfRange)
        );
    }
    for handicap in [i16::MIN, i16::MAX] {
        let result = calculate_handicap(i64::from(handicap) * 1130, 100, true).unwrap();
        assert_eq!(result.playing_handicap, handicap);
    }
}

#[test]
fn zero_allowance_and_disabled_handicaps_have_different_snapshot_semantics() {
    let zero = calculate_handicap(22_600, 0, true).unwrap();
    assert_eq!(zero.course_handicap, 20);
    assert_eq!(zero.playing_handicap, 0);
    let disabled = calculate_handicap(i64::MAX, 85, false).unwrap();
    assert_eq!(disabled.course_handicap, 0);
    assert_eq!(disabled.playing_handicap, 0);
}

#[test]
fn validates_numeric_inputs_without_conflating_missing_states() {
    for value in [i32::MIN, -1, 0, 21, i32::MAX] {
        assert_eq!(GrossScore::new(value), Err(InvalidGrossScore));
    }
    for value in 1..=20 {
        assert_eq!(GrossScore::new(value).unwrap().value(), value);
    }
    assert_ne!(PlayerHoleInput::NoScore, PlayerHoleInput::Unentered);
}

#[test]
fn gross_and_net_choose_different_partners() {
    let result = aggregate_hole([player(A, 0, numeric(4)), player(B, 20, numeric(5))], 1)
        .unwrap()
        .unwrap();
    assert_eq!(
        result.gross,
        SelectedScore {
            score: 4,
            player_ids: vec![A]
        }
    );
    assert_eq!(
        result.net,
        SelectedScore {
            score: 3,
            player_ids: vec![B]
        }
    );
}

#[test]
fn all_equal_winners_are_stable_when_partner_order_changes() {
    let partners = [player(B, 0, numeric(4)), player(A, 18, numeric(5))];
    let first = aggregate_hole(partners, 2).unwrap().unwrap();
    let reverse = aggregate_hole([partners[1], partners[0]], 2)
        .unwrap()
        .unwrap();
    assert_eq!(first, reverse);
    assert_eq!(first.gross.player_ids, vec![B]);
    assert_eq!(
        first.net,
        SelectedScore {
            score: 4,
            player_ids: vec![A, B]
        }
    );
    let tied = aggregate_hole([player(B, 0, numeric(4)), player(A, 0, numeric(4))], 2)
        .unwrap()
        .unwrap();
    assert_eq!(tied.gross.player_ids, vec![A, B]);
    assert_eq!(tied.gross, tied.net);
}

#[test]
fn one_numeric_result_counts_but_two_missing_results_do_not() {
    for missing in [PlayerHoleInput::NoScore, PlayerHoleInput::Unentered] {
        let result = aggregate_hole([player(A, 0, missing), player(B, 36, numeric(6))], 3)
            .unwrap()
            .unwrap();
        assert_eq!(result.gross.score, 6);
        assert_eq!(result.net.score, 4);
        assert_eq!(result.gross.player_ids, vec![B]);
        for other in [PlayerHoleInput::NoScore, PlayerHoleInput::Unentered] {
            assert_eq!(
                aggregate_hole([player(A, 0, missing), player(B, 36, other)], 3),
                Ok(None)
            );
        }
    }
}

#[test]
fn plus_handicaps_give_strokes_back_on_highest_indexes() {
    for (index, expected_net) in [(1, 4), (16, 4), (17, 5), (18, 5)] {
        let result = aggregate_hole(
            [
                player(A, -2, numeric(4)),
                player(B, 0, PlayerHoleInput::NoScore),
            ],
            index,
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.net.score, expected_net);
    }
}

#[test]
fn all_signed_snapshot_allocations_sum_to_the_preserved_handicap() {
    for handicap in [i16::MIN, -40, -19, -18, -2, 0, 2, 18, 19, 40, i16::MAX] {
        let result = aggregate_card(
            [
                partner(A, handicap, numeric(4)),
                partner(B, 0, PlayerHoleInput::Unentered),
            ],
            INDEXES,
        )
        .unwrap();
        let totals = result.totals.unwrap();
        assert_eq!(totals.gross, 72);
        assert_eq!(totals.net, 72 - i32::from(handicap));
        assert_eq!(result.scored_holes, 18);
        assert!(result.is_complete());
    }
}

#[test]
fn negative_net_is_a_valid_side_result() {
    let result = aggregate_hole([player(A, 54, numeric(1)), player(B, 0, numeric(4))], 1)
        .unwrap()
        .unwrap();
    assert_eq!(result.net.score, -2);
}

#[test]
fn partial_card_uses_full_layout_and_counts_side_holes_only() {
    // This card tests aggregation of already-preserved handicaps on one layout.
    let mut a = partner(A, 1, PlayerHoleInput::Unentered);
    let mut b = partner(B, 37, PlayerHoleInput::Unentered);
    a.holes[0] = numeric(4);
    a.holes[1] = numeric(5);
    a.holes[2] = PlayerHoleInput::NoScore;
    b.holes[0] = numeric(5);
    b.holes[1] = numeric(6);
    b.holes[2] = numeric(6);
    // SI2: A0/B2 ->4/3; SI1: A1/B3 ->5/3; SI3: B2 ->6/4.
    let mut indexes = INDEXES;
    indexes.swap(0, 1);
    let result = aggregate_card([a, b], indexes).unwrap();
    assert_eq!(result.totals, Some(SideTotals { gross: 15, net: 10 }));
    assert_eq!(result.scored_holes, 3);
    assert!(!result.is_complete());
    b.holes[2] = PlayerHoleInput::NoScore;
    let result = aggregate_card([a, b], indexes).unwrap();
    assert_eq!(result.scored_holes, 2);
    assert_eq!(result.totals, Some(SideTotals { gross: 9, net: 6 }));
}

#[test]
fn full_card_can_be_complete_with_one_partner_entirely_missing() {
    let result = aggregate_card(
        [
            partner(A, 0, numeric(4)),
            partner(B, 36, PlayerHoleInput::NoScore),
        ],
        INDEXES,
    )
    .unwrap();
    assert!(result.is_complete());
    assert_eq!(result.totals, Some(SideTotals { gross: 72, net: 72 }));
    let blank = aggregate_card(
        [
            partner(A, 0, PlayerHoleInput::Unentered),
            partner(B, 36, PlayerHoleInput::NoScore),
        ],
        INDEXES,
    )
    .unwrap();
    assert_eq!(blank.scored_holes, 0);
    assert_eq!(blank.totals, None);
    assert!(!blank.is_complete());
}

#[test]
fn rejects_duplicate_partners_and_invalid_full_layouts_even_on_empty_cards() {
    let a = partner(A, 0, PlayerHoleInput::Unentered);
    let b = partner(B, 0, PlayerHoleInput::Unentered);
    assert_eq!(
        aggregate_card([a, a], INDEXES),
        Err(FourBallError::DuplicatePlayer)
    );
    let hole = player(A, 0, PlayerHoleInput::Unentered);
    assert_eq!(
        aggregate_hole([hole, hole], 1),
        Err(FourBallError::DuplicatePlayer)
    );
    for invalid in [0, 19, u8::MAX] {
        assert_eq!(
            aggregate_hole([hole, player(B, 0, PlayerHoleInput::NoScore)], invalid),
            Err(FourBallError::InvalidStrokeIndex)
        );
    }
    for invalid in [0, 1, 19, u8::MAX] {
        let mut indexes = INDEXES;
        indexes[17] = invalid;
        assert_eq!(
            aggregate_card([a, b], indexes),
            Err(FourBallError::InvalidStrokeIndex)
        );
    }
}

#[test]
fn format_remains_rejected_by_existing_transport_enum() {
    assert!(serde_json::from_str::<ScoringFormat>("\"four_ball\"").is_err());
    assert!(serde_json::from_str::<ScoringFormat>("\"four_ball_stroke_play\"").is_err());
}

#[test]
fn contract_hole_examples_have_independent_winners_and_expected_subtotal() {
    // Each contract row specifies already-calculated hole strokes independently.
    let cases = [
        (
            [player(A, 0, numeric(4)), player(B, 20, numeric(5))],
            1,
            4,
            3,
        ),
        (
            [player(A, 18, numeric(5)), player(B, 0, numeric(4))],
            2,
            4,
            4,
        ),
        (
            [
                player(A, 0, PlayerHoleInput::NoScore),
                player(B, 36, numeric(6)),
            ],
            3,
            6,
            4,
        ),
    ];
    let mut gross = 0;
    let mut net = 0;
    for (partners, index, expected_gross, expected_net) in cases {
        let hole = aggregate_hole(partners, index).unwrap().unwrap();
        assert_eq!(
            (hole.gross.score, hole.net.score),
            (expected_gross, expected_net)
        );
        gross += hole.gross.score;
        net += hole.net.score;
    }
    assert_eq!((gross, net), (14, 11));
    assert_eq!((gross - 12, net - 12), (2, -1));
}

#[test]
fn partial_front_nine_does_not_redistribute_the_handicap() {
    let mut a = partner(A, 20, PlayerHoleInput::Unentered);
    for input in a.holes.iter_mut().take(9) {
        *input = numeric(4);
    }
    let card = aggregate_card([a, partner(B, 0, PlayerHoleInput::NoScore)], INDEXES).unwrap();
    assert_eq!(card.scored_holes, 9);
    assert_eq!(card.totals, Some(SideTotals { gross: 36, net: 25 }));
    assert!(!card.is_complete());
}
