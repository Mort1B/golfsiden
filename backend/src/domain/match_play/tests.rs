use super::*;
use crate::domain::{models::ScoringFormat, player_score_input::GrossScore};

fn reports(outcomes: &[HoleOutcome]) -> Vec<MatchReport> {
    outcomes
        .iter()
        .enumerate()
        .map(|(index, outcome)| MatchReport::Hole {
            number: index as u8 + 1,
            outcome: *outcome,
        })
        .collect()
}

fn score(value: i32) -> GrossScore {
    GrossScore::new(value).unwrap()
}

fn award(state: MatchState) -> (u32, u32) {
    let award = state
        .point_award(ConfirmationStatus::Confirmed)
        .unwrap()
        .unwrap();
    (award.first.half_units(), award.second.half_units())
}

#[test]
fn default_net_uses_full_signed_handicap_difference() {
    assert_eq!(MatchMode::default(), MatchMode::Net);
    for (inputs, expected) in [
        ([10, 18], [0, 8]),
        ([-2, 14], [0, 16]),
        ([-4, -1], [0, 3]),
        ([14, -2], [16, 0]),
        ([12, 12], [0, 0]),
    ] {
        assert_eq!(
            HandicapAllocation::new(MatchMode::Net, inputs).relative_handicaps(),
            expected
        );
    }
}

#[test]
fn gross_mode_ignores_handicaps_and_net_mode_can_change_the_proposal() {
    let gross = HandicapAllocation::new(MatchMode::Gross, [10, 18]);
    let net = HandicapAllocation::new(MatchMode::Net, [10, 18]);
    assert_eq!(gross.relative_handicaps(), [0, 0]);
    assert_eq!(
        gross.propose_numeric_hole([score(5), score(5)], 3),
        Ok(HoleOutcome::Halved)
    );
    assert_eq!(
        net.propose_numeric_hole([score(5), score(5)], 3),
        Ok(HoleOutcome::WonBy(Opponent::Second))
    );
    assert_eq!(
        net.propose_numeric_hole([score(5), score(5)], 9),
        Ok(HoleOutcome::Halved)
    );
}

#[test]
fn allocation_uses_index_not_hole_number_and_supports_more_than_eighteen_strokes() {
    let eight = HandicapAllocation::new(MatchMode::Net, [10, 18]);
    for index in 1..=18 {
        assert_eq!(
            eight.strokes_for_hole(index),
            Ok([0, i32::from(index <= 8)])
        );
    }
    let forty = HandicapAllocation::new(MatchMode::Net, [0, 40]);
    for index in 1..=18 {
        assert_eq!(
            forty.strokes_for_hole(index),
            Ok([0, if index <= 4 { 3 } else { 2 }])
        );
    }
}

#[test]
fn signed_extremes_widen_before_subtraction_and_allocate_exactly() {
    for inputs in [
        [i16::MIN, i16::MAX],
        [i16::MAX, i16::MIN],
        [-4, -1],
        [-20, 20],
        [20, 20],
    ] {
        let allocation = HandicapAllocation::new(MatchMode::Net, inputs);
        let mut sums = [0, 0];
        for index in 1..=18 {
            let [first, second] = allocation.strokes_for_hole(index).unwrap();
            sums[0] += first;
            sums[1] += second;
        }
        assert_eq!(sums, allocation.relative_handicaps());
    }
    let extreme = HandicapAllocation::new(MatchMode::Net, [i16::MIN, i16::MAX]);
    assert_eq!(extreme.relative_handicaps(), [0, 65535]);
    assert_eq!(
        extreme.propose_numeric_hole([score(1), score(20)], 1),
        Ok(HoleOutcome::WonBy(Opponent::Second))
    );
}

#[test]
fn swapping_opponents_mirrors_numeric_proposals() {
    for mode in [MatchMode::Gross, MatchMode::Net] {
        for index in 1..=18 {
            for first_score in 1..=20 {
                let first = HandicapAllocation::new(mode, [-2, 14])
                    .propose_numeric_hole([score(first_score), score(5)], index)
                    .unwrap();
                let second = HandicapAllocation::new(mode, [14, -2])
                    .propose_numeric_hole([score(5), score(first_score)], index)
                    .unwrap();
                let mirrored = match first {
                    HoleOutcome::WonBy(winner) => HoleOutcome::WonBy(winner.other()),
                    HoleOutcome::Halved => HoleOutcome::Halved,
                };
                assert_eq!(second, mirrored);
            }
        }
    }
}

#[test]
fn invalid_stroke_indexes_fail_in_both_modes() {
    for mode in [MatchMode::Gross, MatchMode::Net] {
        let allocation = HandicapAllocation::new(mode, [0, 40]);
        for index in [0, 19, u8::MAX] {
            assert_eq!(
                allocation.strokes_for_hole(index),
                Err(MatchPlayError::InvalidHole)
            );
            assert_eq!(
                allocation.propose_numeric_hole([score(4), score(5)], index),
                Err(MatchPlayError::InvalidHole)
            );
        }
    }
}

#[test]
fn empty_match_has_no_result_or_points() {
    let state = derive_match(&[]).unwrap();
    assert_eq!(state.lead(), 0);
    assert_eq!(state.resolved_holes(), 0);
    assert_eq!(state.holes_remaining(), 18);
    assert_eq!(state.finish(), None);
    assert_eq!(state.point_award(ConfirmationStatus::Unconfirmed), Ok(None));
    assert_eq!(
        state.point_award(ConfirmationStatus::Confirmed),
        Err(MatchPlayError::UnfinishedConfirmation)
    );
}

#[test]
fn contract_three_and_two_finishes_after_sixteen_for_either_opponent() {
    for winner in [Opponent::First, Opponent::Second] {
        let mut holes = vec![HoleOutcome::Halved; 13];
        holes.extend([HoleOutcome::WonBy(winner); 3]);
        let state = derive_match(&reports(&holes)).unwrap();
        assert_eq!(state.resolved_holes(), 16);
        assert_eq!(state.holes_remaining(), 2);
        assert_eq!(
            state.finish(),
            Some(MatchFinish::OnHoles {
                winner,
                margin: 3,
                holes_remaining: 2
            })
        );
        assert_eq!(state.lead(), if winner == Opponent::First { 3 } else { -3 });
        assert_eq!(
            award(state),
            if winner == Opponent::First {
                (2, 0)
            } else {
                (0, 2)
            }
        );
    }
}

#[test]
fn two_up_with_two_left_is_not_terminal_and_can_be_drawn() {
    let mut holes = vec![HoleOutcome::Halved; 14];
    holes.extend([HoleOutcome::WonBy(Opponent::First); 2]);
    let state = derive_match(&reports(&holes)).unwrap();
    assert_eq!(state.lead(), 2);
    assert_eq!(state.holes_remaining(), 2);
    assert_eq!(state.finish(), None);
    assert_eq!(
        state.point_award(ConfirmationStatus::Confirmed),
        Err(MatchPlayError::UnfinishedConfirmation)
    );
    let mut halved_seventeen = holes.clone();
    halved_seventeen.push(HoleOutcome::Halved);
    assert_eq!(
        derive_match(&reports(&halved_seventeen)).unwrap().finish(),
        Some(MatchFinish::OnHoles {
            winner: Opponent::First,
            margin: 2,
            holes_remaining: 1
        })
    );
    holes.extend([HoleOutcome::WonBy(Opponent::Second); 2]);
    assert_eq!(
        derive_match(&reports(&holes)).unwrap().finish(),
        Some(MatchFinish::Draw)
    );
}

#[test]
fn earliest_hole_win_is_ten_and_eight_not_nine_and_nine() {
    let mut holes = vec![HoleOutcome::WonBy(Opponent::First); 9];
    assert_eq!(derive_match(&reports(&holes)).unwrap().finish(), None);
    holes.push(HoleOutcome::WonBy(Opponent::First));
    assert_eq!(
        derive_match(&reports(&holes)).unwrap().finish(),
        Some(MatchFinish::OnHoles {
            winner: Opponent::First,
            margin: 10,
            holes_remaining: 8
        })
    );
}

#[test]
fn final_hole_can_finish_one_or_two_up() {
    for winner in [Opponent::First, Opponent::Second] {
        for previous_wins in [0, 1] {
            let mut holes = vec![HoleOutcome::Halved; 17 - previous_wins];
            holes.extend(vec![HoleOutcome::WonBy(winner); previous_wins + 1]);
            let state = derive_match(&reports(&holes)).unwrap();
            assert_eq!(state.resolved_holes(), 18);
            assert_eq!(
                state.finish(),
                Some(MatchFinish::OnHoles {
                    winner,
                    margin: previous_wins as u8 + 1,
                    holes_remaining: 0
                })
            );
        }
    }
}

#[test]
fn eighteen_halves_draw_without_extra_holes_or_provisional_points() {
    let sequence = reports(&[HoleOutcome::Halved; 18]);
    let state = derive_match(&sequence).unwrap();
    assert_eq!(state.finish(), Some(MatchFinish::Draw));
    assert_eq!(state.lead(), 0);
    assert_eq!(state.holes_remaining(), 0);
    assert_eq!(state.point_award(ConfirmationStatus::Unconfirmed), Ok(None));
    assert_eq!(award(state), (1, 1));
    let mut extra_hole = sequence;
    extra_hole.push(MatchReport::Hole {
        number: 19,
        outcome: HoleOutcome::WonBy(Opponent::First),
    });
    assert_eq!(
        derive_match(&extra_hole),
        Err(MatchPlayError::MatchAlreadyFinished)
    );
}

#[test]
fn concessions_before_play_or_mid_match_do_not_invent_margin_or_holes() {
    for conceder in [Opponent::First, Opponent::Second] {
        for resolved in [0, 9] {
            let mut sequence = reports(&vec![HoleOutcome::Halved; resolved]);
            sequence.push(MatchReport::Conceded { by: conceder });
            let state = derive_match(&sequence).unwrap();
            assert_eq!(state.resolved_holes(), resolved as u8);
            assert_eq!(
                state.finish(),
                Some(MatchFinish::Conceded {
                    winner: conceder.other()
                })
            );
            assert_eq!(state.point_award(ConfirmationStatus::Unconfirmed), Ok(None));
            assert_eq!(
                award(state),
                if conceder == Opponent::First {
                    (0, 2)
                } else {
                    (2, 0)
                }
            );
        }
    }
}

#[test]
fn organizer_award_uses_explicit_winner_not_current_leader() {
    let sequence = [
        MatchReport::Hole {
            number: 1,
            outcome: HoleOutcome::WonBy(Opponent::First),
        },
        MatchReport::Awarded {
            winner: Opponent::Second,
        },
    ];
    let state = derive_match(&sequence).unwrap();
    assert_eq!(state.lead(), 1);
    assert_eq!(state.resolved_holes(), 1);
    assert_eq!(
        state.finish(),
        Some(MatchFinish::Awarded {
            winner: Opponent::Second
        })
    );
    assert_eq!(award(state), (0, 2));
}

#[test]
fn rejects_gaps_duplicates_out_of_order_and_invalid_holes() {
    for number in [0, 19, u8::MAX] {
        assert_eq!(
            derive_match(&[MatchReport::Hole {
                number,
                outcome: HoleOutcome::Halved
            }]),
            Err(MatchPlayError::InvalidHole)
        );
    }
    for numbers in [vec![2], vec![1, 1], vec![1, 3], vec![1, 2, 1]] {
        let sequence: Vec<_> = numbers
            .into_iter()
            .map(|number| MatchReport::Hole {
                number,
                outcome: HoleOutcome::Halved,
            })
            .collect();
        assert_eq!(derive_match(&sequence), Err(MatchPlayError::OutOfOrderHole));
    }
}

#[test]
fn every_terminal_kind_rejects_any_trailing_report() {
    let terminal_sequences = [
        reports(&[HoleOutcome::WonBy(Opponent::First); 10]),
        reports(&[HoleOutcome::Halved; 18]),
        vec![MatchReport::Conceded {
            by: Opponent::First,
        }],
        vec![MatchReport::Awarded {
            winner: Opponent::First,
        }],
    ];
    for sequence in terminal_sequences {
        for trailing in [
            MatchReport::Hole {
                number: 1,
                outcome: HoleOutcome::Halved,
            },
            MatchReport::Conceded {
                by: Opponent::Second,
            },
            MatchReport::Awarded {
                winner: Opponent::Second,
            },
        ] {
            let mut invalid = sequence.clone();
            invalid.push(trailing);
            assert_eq!(
                derive_match(&invalid),
                Err(MatchPlayError::MatchAlreadyFinished)
            );
        }
    }
}

#[test]
fn numeric_proposals_cannot_recompute_accepted_hole_outcomes() {
    let accepted = [MatchReport::Hole {
        number: 1,
        outcome: HoleOutcome::WonBy(Opponent::First),
    }];
    let original = derive_match(&accepted).unwrap();
    let new_proposal = HandicapAllocation::new(MatchMode::Net, [10, 18])
        .propose_numeric_hole([score(5), score(5)], 1)
        .unwrap();
    assert_eq!(new_proposal, HoleOutcome::WonBy(Opponent::Second));
    assert_eq!(derive_match(&accepted).unwrap(), original);
    assert_eq!(original.lead(), 1);
}

#[test]
fn win_draw_loss_add_to_exactly_one_and_a_half_points() {
    let won = derive_match(&[MatchReport::Awarded {
        winner: Opponent::First,
    }])
    .unwrap()
    .point_award(ConfirmationStatus::Confirmed)
    .unwrap()
    .unwrap();
    let drawn = derive_match(&reports(&[HoleOutcome::Halved; 18]))
        .unwrap()
        .point_award(ConfirmationStatus::Confirmed)
        .unwrap()
        .unwrap();
    let lost = derive_match(&[MatchReport::Conceded {
        by: Opponent::First,
    }])
    .unwrap()
    .point_award(ConfirmationStatus::Confirmed)
    .unwrap()
    .unwrap();
    assert_eq!(
        won.first
            .checked_add(drawn.first)
            .unwrap()
            .checked_add(lost.first)
            .unwrap()
            .half_units(),
        3
    );
    assert_eq!(MatchPoints::default().half_units(), 0);
    for award in [won, drawn, lost] {
        assert_eq!(
            award.first.checked_add(award.second).unwrap().half_units(),
            2
        );
    }
}

#[test]
fn permitted_prefix_is_independent_of_different_hidden_finishes() {
    let mut first = vec![HoleOutcome::Halved; 9];
    let mut second = first.clone();
    first.extend([HoleOutcome::WonBy(Opponent::First); 5]);
    second.extend([HoleOutcome::WonBy(Opponent::Second); 5]);
    let first = reports(&first);
    let second = reports(&second);
    assert_ne!(
        derive_match(&first).unwrap(),
        derive_match(&second).unwrap()
    );
    let prefix = derive_match(&first[..9]).unwrap();
    assert_eq!(prefix, derive_match(&second[..9]).unwrap());
    assert_eq!(prefix.finish(), None);
    assert_eq!(
        prefix.point_award(ConfirmationStatus::Unconfirmed),
        Ok(None)
    );
}

#[test]
fn only_supported_singles_format_is_available_in_transport() {
    for name in ["match_play", "partner_match_play"] {
        assert!(
            serde_json::from_value::<ScoringFormat>(serde_json::Value::String(name.into()))
                .is_err()
        );
    }
}

#[test]
fn singles_transport_format_is_explicit() {
    assert_eq!(
        serde_json::from_value::<ScoringFormat>(serde_json::json!("singles_match_play")).unwrap(),
        ScoringFormat::SinglesMatchPlay
    );
}
