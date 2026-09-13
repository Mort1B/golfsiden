use super::*;
use crate::domain::{models::ScoringFormat, player_score_input::InvalidGrossScore};

fn numeric(strokes: i32) -> PlayerHoleInput {
    PlayerHoleInput::Numeric(GrossScore::new(strokes).unwrap())
}

fn layout() -> [HoleLayout; 18] {
    std::array::from_fn(|index| HoleLayout {
        par: 4,
        stroke_index: index as u8 + 1,
    })
}

fn hole(input: PlayerHoleInput, par: u8, handicap: i16, index: u8) -> Option<HoleResult> {
    calculate_hole(
        input,
        HoleLayout {
            par,
            stroke_index: index,
        },
        handicap,
    )
    .unwrap()
}

fn totals(card: &CardResult) -> (i32, i32, i32, i32) {
    let result = card.totals.unwrap();
    (
        result.points.gross.value(),
        result.points.net.value(),
        result.overall.gross.value(),
        result.overall.net.value(),
    )
}

#[test]
fn contract_hole_examples_and_six_hole_arithmetic() {
    let cases = [
        (4, numeric(4), 0, 2, 2),
        (4, numeric(5), 18, 1, 2),
        (4, numeric(6), 36, 0, 2),
        (4, numeric(9), 18, 0, 0),
        (3, numeric(2), 0, 3, 3),
        (5, PlayerHoleInput::NoScore, 36, 0, 0),
    ];
    let mut gross = 0;
    let mut net = 0;
    for (par, input, handicap, expected_gross, expected_net) in cases {
        let result = hole(input, par, handicap, 1).unwrap();
        assert_eq!(result.points.gross.value(), expected_gross);
        assert_eq!(result.points.net.value(), expected_net);
        gross += result.points.gross.value();
        net += result.points.net.value();
    }
    assert_eq!((gross, net), (6, 9));
    assert_eq!((12 - gross, 12 - net), (6, 3));
}

#[test]
fn ordinary_points_mapping_floors_only_at_zero() {
    for (strokes, expected) in [
        (1, 5),
        (2, 4),
        (3, 3),
        (4, 2),
        (5, 1),
        (6, 0),
        (7, 0),
        (20, 0),
    ] {
        let result = hole(numeric(strokes), 4, 0, 1).unwrap();
        assert_eq!(result.points.gross.value(), expected);
        assert_eq!(
            result.points,
            GrossNet {
                gross: result.points.gross,
                net: result.points.gross
            }
        );
        assert_eq!(result.strokes.unwrap().gross.value(), strokes);
    }
}

#[test]
fn gross_zero_can_still_earn_net_points_but_pickup_cannot() {
    let numeric = hole(numeric(6), 4, 36, 1).unwrap();
    assert_eq!(numeric.points.gross.value(), 0);
    assert_eq!(numeric.points.net.value(), 2);
    let pickup = hole(PlayerHoleInput::NoScore, 4, 36, 1).unwrap();
    assert_eq!(pickup.points.gross.value(), 0);
    assert_eq!(pickup.points.net.value(), 0);
    assert_eq!(pickup.strokes, None);
    assert_eq!(hole(PlayerHoleInput::Unentered, 4, 36, 1), None);
}

#[test]
fn negative_adjusted_strokes_allow_more_than_six_points() {
    let result = hole(numeric(1), 5, 54, 1).unwrap();
    assert_eq!(result.strokes.unwrap().net.value(), -2);
    assert_eq!(result.points.gross.value(), 6);
    assert_eq!(result.points.net.value(), 9);
}

#[test]
fn plus_handicap_strokes_are_given_back_at_highest_indexes() {
    for (index, expected) in [(1, 2), (16, 2), (17, 1), (18, 1)] {
        let result = hole(numeric(4), 4, -2, index).unwrap();
        assert_eq!(result.points.gross.value(), 2);
        assert_eq!(result.points.net.value(), expected);
    }
}

#[test]
fn handicap_is_applied_before_each_holes_points_floor() {
    let card = calculate_card([numeric(10); 18], layout(), 36).unwrap();
    assert_eq!(totals(&card), (0, 0, 36, 36));
    assert_ne!(
        card.totals.unwrap().points.net.value(),
        card.totals.unwrap().points.gross.value() + 36
    );
    assert_eq!(card.full_stroke_totals.unwrap().net.value(), 144);
}

#[test]
fn blank_pickup_and_numeric_zero_cards_have_distinct_results() {
    let blank = calculate_card([PlayerHoleInput::Unentered; 18], layout(), 0).unwrap();
    assert_eq!(blank.resolved_holes, 0);
    assert_eq!(blank.totals, None);
    assert_eq!(blank.full_stroke_totals, None);
    assert!(!blank.is_complete());
    let pickup = calculate_card([PlayerHoleInput::NoScore; 18], layout(), 0).unwrap();
    assert_eq!(pickup.resolved_holes, 18);
    assert_eq!(totals(&pickup), (0, 0, 36, 36));
    assert_eq!(pickup.full_stroke_totals, None);
    assert!(pickup.is_complete());
    let numeric = calculate_card([numeric(6); 18], layout(), 0).unwrap();
    assert_eq!(totals(&numeric), totals(&pickup));
    assert_eq!(numeric.full_stroke_totals.unwrap().gross.value(), 108);
    assert!(numeric.is_complete());
}

#[test]
fn one_resolved_zero_hole_creates_a_partial_result() {
    let mut inputs = [PlayerHoleInput::Unentered; 18];
    inputs[17] = PlayerHoleInput::NoScore;
    let card = calculate_card(inputs, layout(), 20).unwrap();
    assert_eq!(card.resolved_holes, 1);
    assert_eq!(totals(&card), (0, 0, 2, 2));
    assert!(!card.is_complete());
    assert_eq!(card.full_stroke_totals, None);
}

#[test]
fn incomplete_numeric_cards_have_no_full_actual_total() {
    let mut inputs = [numeric(4); 18];
    inputs[17] = PlayerHoleInput::Unentered;
    let card = calculate_card(inputs, layout(), 0).unwrap();
    assert_eq!(card.resolved_holes, 17);
    assert_eq!(totals(&card), (34, 34, 0, 0));
    assert_eq!(card.full_stroke_totals, None);
    assert!(!card.is_complete());
}

#[test]
fn partial_card_baseline_counts_resolved_holes_including_pickups() {
    let mut inputs = [PlayerHoleInput::Unentered; 18];
    for (slot, score) in inputs.iter_mut().zip([4, 5, 6, 9, 2]) {
        *slot = numeric(score);
    }
    inputs[5] = PlayerHoleInput::NoScore;
    let mut course = layout();
    course[4].par = 3;
    course[5].par = 5;
    // One preserved handicap allocates strokes on indexes 1, 2 and 3.
    let card = calculate_card(inputs, course, 3).unwrap();
    assert_eq!(card.resolved_holes, 6);
    assert_eq!(totals(&card), (6, 9, 6, 3));
    inputs[5] = PlayerHoleInput::Unentered;
    let card = calculate_card(inputs, course, 3).unwrap();
    assert_eq!(card.resolved_holes, 5);
    assert_eq!(totals(&card), (6, 9, 4, 1));
}

#[test]
fn full_par_and_forty_point_rounds_use_thirty_six_minus_points() {
    let mut inputs = [numeric(4); 18];
    let pars = calculate_card(inputs, layout(), 0).unwrap();
    assert_eq!(totals(&pars), (36, 36, 0, 0));
    assert_eq!(pars.full_stroke_totals.unwrap().gross.value(), 72);
    for input in inputs.iter_mut().take(4) {
        *input = numeric(3);
    }
    let birdies = calculate_card(inputs, layout(), 0).unwrap();
    assert_eq!(totals(&birdies), (40, 40, -4, -4));
    assert_eq!(birdies.full_stroke_totals.unwrap().gross.value(), 68);
    assert!(birdies.is_complete());
}

#[test]
fn bad_hole_cap_is_not_an_actual_stroke_total() {
    let mut inputs = [numeric(4); 18];
    inputs[0] = numeric(10);
    let numeric = calculate_card(inputs, layout(), 0).unwrap();
    assert_eq!(totals(&numeric), (34, 34, 2, 2));
    assert_eq!(numeric.full_stroke_totals.unwrap().gross.value(), 78);
    inputs[0] = PlayerHoleInput::NoScore;
    let pickup = calculate_card(inputs, layout(), 0).unwrap();
    assert_eq!(totals(&pickup), totals(&numeric));
    assert_eq!(pickup.full_stroke_totals, None);
    assert!(pickup.is_complete());
}

#[test]
fn numeric_corrections_are_preserved_even_when_points_do_not_change() {
    let nine = hole(numeric(9), 4, 0, 1).unwrap();
    let ten = hole(numeric(10), 4, 0, 1).unwrap();
    assert_eq!(nine.points, ten.points);
    assert_ne!(nine, ten);
    assert_eq!(nine.strokes.unwrap().gross.value(), 9);
    assert_eq!(ten.strokes.unwrap().gross.value(), 10);
}

#[test]
fn hidden_inputs_cannot_change_any_projected_field() {
    let mut inputs = [numeric(4); 18];
    inputs[0] = numeric(3);
    inputs[1] = numeric(3);
    let mask = std::array::from_fn(|index| index < 9);
    let original = project_card(inputs, layout(), 20, mask).unwrap();
    assert_eq!(original.visible_hole_count, 9);
    assert_eq!(original.resolved_holes, 9);
    // Net adds 11 strokes allocated to the full round's indexes 1..9.
    assert_eq!(totals(&original), (20, 31, -2, -13));
    assert_eq!(original.full_stroke_totals, None);
    assert!(!original.is_complete());
    for hidden in [
        PlayerHoleInput::Unentered,
        PlayerHoleInput::NoScore,
        numeric(1),
        numeric(20),
    ] {
        for input in inputs.iter_mut().skip(9) {
            *input = hidden;
        }
        assert_eq!(project_card(inputs, layout(), 20, mask).unwrap(), original);
    }
}

#[test]
fn all_hidden_and_visible_blanks_have_no_contribution() {
    for input in [
        PlayerHoleInput::Unentered,
        PlayerHoleInput::NoScore,
        numeric(1),
        numeric(20),
    ] {
        let hidden = project_card([input; 18], layout(), i16::MAX, [false; 18]).unwrap();
        assert_eq!(hidden.visible_hole_count, 0);
        assert_eq!(hidden.resolved_holes, 0);
        assert_eq!(hidden.holes, [None; 18]);
        assert_eq!(hidden.totals, None);
        assert_eq!(hidden.full_stroke_totals, None);
        assert!(!hidden.is_complete());
        let mut inputs = [input; 18];
        inputs[0] = PlayerHoleInput::Unentered;
        let first_only =
            project_card(inputs, layout(), 0, std::array::from_fn(|i| i == 0)).unwrap();
        assert_eq!(first_only.totals, None);
        assert_eq!(first_only.resolved_holes, 0);
    }
}

#[test]
fn full_projection_matches_full_card_and_layout_order_is_respected() {
    let inputs = [numeric(4); 18];
    assert_eq!(
        project_card(inputs, layout(), -2, [true; 18]),
        calculate_card(inputs, layout(), -2)
    );
    let mut course = layout();
    course.swap(0, 17);
    let first_only = project_card(inputs, course, -2, std::array::from_fn(|i| i == 0)).unwrap();
    assert_eq!(totals(&first_only), (2, 1, 0, 1));
}

#[test]
fn signed_snapshot_extremes_and_points_sums_remain_bounded() {
    let high = calculate_card([numeric(1); 18], layout(), i16::MAX).unwrap();
    assert_eq!(
        totals(&high),
        (90, 90 + i32::from(i16::MAX), -54, -54 - i32::from(i16::MAX))
    );
    assert_eq!(
        high.full_stroke_totals.unwrap().net.value(),
        18 - i32::from(i16::MAX)
    );
    let low = calculate_card([numeric(20); 18], layout(), i16::MIN).unwrap();
    assert_eq!(totals(&low), (0, 0, 36, 36));
    assert_eq!(
        low.full_stroke_totals.unwrap().net.value(),
        360 - i32::from(i16::MIN)
    );
}

#[test]
fn validates_par_stroke_indexes_and_shared_numeric_range() {
    for par in [0, 1, 8, u8::MAX] {
        let mut course = layout();
        course[17].par = par;
        assert_eq!(
            project_card([PlayerHoleInput::Unentered; 18], course, 0, [false; 18]),
            Err(StablefordError::InvalidPar)
        );
    }
    for par in [2, 7] {
        assert!(
            calculate_hole(
                numeric(1),
                HoleLayout {
                    par,
                    stroke_index: 1
                },
                0
            )
            .is_ok()
        );
    }
    for index in [0, 1, 19, u8::MAX] {
        let mut course = layout();
        course[17].stroke_index = index;
        assert_eq!(
            calculate_card([PlayerHoleInput::Unentered; 18], course, 0),
            Err(StablefordError::InvalidStrokeIndex)
        );
    }
    for index in [0, 19, u8::MAX] {
        assert_eq!(
            calculate_hole(
                PlayerHoleInput::NoScore,
                HoleLayout {
                    par: 4,
                    stroke_index: index
                },
                0
            ),
            Err(StablefordError::InvalidStrokeIndex)
        );
    }
    for value in [i32::MIN, 0, 21, i32::MAX] {
        assert_eq!(GrossScore::new(value), Err(InvalidGrossScore));
    }
}

#[test]
fn native_points_and_overall_ordering_are_opposite_for_complete_cards() {
    let mut inputs = [numeric(4); 18];
    inputs[0] = numeric(3);
    inputs[1] = numeric(3);
    let lower = calculate_card(inputs, layout(), 0).unwrap().totals.unwrap();
    inputs[2] = numeric(3);
    inputs[3] = numeric(3);
    let higher = calculate_card(inputs, layout(), 0).unwrap().totals.unwrap();
    assert_eq!(
        (lower.points.gross.value(), higher.points.gross.value()),
        (38, 40)
    );
    assert!(higher.points.gross.value() > lower.points.gross.value());
    assert!(higher.overall.gross.value() < lower.overall.gross.value());
}

#[test]
fn format_uses_explicit_individual_transport_name() {
    assert!(serde_json::from_str::<ScoringFormat>("\"stableford\"").is_err());
    assert_eq!(
        serde_json::from_str::<ScoringFormat>("\"individual_stableford\"").unwrap(),
        ScoringFormat::IndividualStableford
    );
}
