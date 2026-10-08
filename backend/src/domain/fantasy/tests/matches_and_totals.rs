use super::*;
use crate::domain::fantasy::{
    matches::*,
    rounds::GolferPoints,
    selections::{Lineup, Origin, Selection},
    totals::*,
};
use crate::domain::match_play::{
    ConfirmationStatus, HoleOutcome, MatchReport, Opponent, derive_match,
};
fn match_source(reports: &[MatchReport], confirmed: bool) -> MatchSource {
    MatchSource::Visible {
        state: derive_match(reports).unwrap(),
        confirmation: if confirmed {
            ConfirmationStatus::Confirmed
        } else {
            ConfirmationStatus::Unconfirmed
        },
        source: token(1),
        non_finish: None,
    }
}
#[test]
fn early_match_finish_scores_only_accepted_outcome() {
    let reports: Vec<_> = (1..=10)
        .map(|number| MatchReport::Hole {
            number,
            outcome: HoleOutcome::WonBy(Opponent::First),
        })
        .collect();
    let source = match_source(&reports, true);
    assert_eq!(match_points(source, Opponent::First), Points::Settled(3));
    assert_eq!(match_points(source, Opponent::Second), Points::Settled(-1));
    assert_eq!(
        match_points(source, Opponent::Second).multiply(2),
        Ok(Points::Settled(-2))
    );
}
#[test]
fn conceded_and_awarded_results_have_same_outcome_points() {
    for report in [
        MatchReport::Conceded {
            by: Opponent::Second,
        },
        MatchReport::Awarded {
            winner: Opponent::First,
        },
    ] {
        assert_eq!(
            match_points(match_source(&[report], true), Opponent::First),
            Points::Settled(3)
        );
        assert_eq!(
            match_points(match_source(&[report], false), Opponent::Second),
            Points::Provisional(-1)
        );
    }
}
#[test]
fn draw_pending_and_withheld_are_distinct() {
    let draw: Vec<_> = (1..=18)
        .map(|number| MatchReport::Hole {
            number,
            outcome: HoleOutcome::Halved,
        })
        .collect();
    for player in [Opponent::First, Opponent::Second] {
        assert_eq!(
            match_points(match_source(&draw, true), player),
            Points::Settled(1)
        );
        assert_eq!(
            match_points(match_source(&[], false), player),
            Points::Pending { recorded: 0 }
        );
        assert_eq!(
            match_points(MatchSource::Withheld, player),
            Points::Withheld
        );
    }
}
#[test]
fn no_result_disposition_must_match_source_but_cannot_override_accepted_loss() {
    for (attested, expected) in [
        (token(1), Points::Settled(0)),
        (token(0), Points::Pending { recorded: 0 }),
    ] {
        let source = MatchSource::Visible {
            state: derive_match(&[]).unwrap(),
            confirmation: ConfirmationStatus::Unconfirmed,
            source: token(1),
            non_finish: Some(attested),
        };
        assert_eq!(match_points(source, Opponent::First), expected);
    }
    let source = MatchSource::Visible {
        state: derive_match(&[MatchReport::Conceded {
            by: Opponent::First,
        }])
        .unwrap(),
        confirmation: ConfirmationStatus::Confirmed,
        source: token(1),
        non_finish: Some(token(1)),
    };
    assert_eq!(match_points(source, Opponent::First), Points::Settled(-1));
}
fn selection(captain: u128) -> Selection {
    Selection::Locked {
        round: round(1),
        lineup: Lineup::new(
            &[golfer(1), golfer(2), golfer(3), golfer(4)],
            golfer(captain),
        )
        .unwrap(),
        origin: Origin::Submitted,
    }
}
fn golfers(values: [Points; 4]) -> Vec<GolferPoints> {
    values
        .into_iter()
        .enumerate()
        .map(|(index, points)| GolferPoints {
            golfer: golfer(index as u128 + 1),
            points,
        })
        .collect()
}
#[test]
fn captain_doubles_signed_whole_result_and_does_not_touch_golfers() {
    let values = golfers([3, 1, -1, 3].map(Points::Settled));
    assert_eq!(
        manager_round(round(1), &selection(3), &values),
        Ok(Some(Points::Settled(5)))
    );
    assert_eq!(values[2].points, Points::Settled(-1));
    let values = golfers([12, 12, 0, 0].map(Points::Settled));
    assert_eq!(
        manager_round(round(1), &selection(1), &values),
        Ok(Some(Points::Settled(36)))
    );
    assert_eq!(Points::Settled(-4).multiply(2), Ok(Points::Settled(-8)));
}
#[test]
fn missing_picks_not_coerced_to_zero_but_missed_lineups_explicit_zero() {
    assert_eq!(
        manager_round(round(1), &selection(1), &[]),
        Ok(Some(Points::Pending { recorded: 0 }))
    );
    for selected in [Selection::Missed, Selection::Invalid] {
        assert_eq!(
            manager_round(round(1), &selected, &[]),
            Ok(Some(Points::Settled(0)))
        );
    }
    assert_eq!(
        manager_round(round(1), &Selection::NotParticipating, &[]),
        Ok(None)
    );
    assert_eq!(
        manager_round(round(2), &selection(1), &[]),
        Err(FantasyError::InvalidFacts)
    );
    let duplicate = [GolferPoints {
        golfer: golfer(1),
        points: Points::Settled(1),
    }; 2];
    assert_eq!(
        manager_round(round(1), &selection(1), &duplicate),
        Err(FantasyError::DuplicateIdentity)
    );
}
#[test]
fn overflow_is_checked_and_withheld_dominates_arithmetic() {
    assert_eq!(
        Points::Settled(i64::MIN).multiply(2),
        Err(FantasyError::Overflow)
    );
    assert_eq!(
        sum_points(&[Points::Settled(i64::MAX), Points::Settled(1)]),
        Err(FantasyError::Overflow)
    );
    assert_eq!(
        sum_points(&[
            Points::Settled(i64::MAX),
            Points::Settled(1),
            Points::Withheld
        ]),
        Ok(Points::Withheld)
    );
    let values = golfers([
        Points::Settled(i64::MAX),
        Points::Withheld,
        Points::Settled(1),
        Points::Settled(1),
    ]);
    assert_eq!(
        manager_round(round(1), &selection(1), &values),
        Ok(Some(Points::Withheld))
    );
}
#[test]
fn all_rounds_sum_and_missing_inventory_cannot_silently_finalize() {
    let contributions = [
        RoundContribution {
            round: round(1),
            points: Points::Settled(12),
        },
        RoundContribution {
            round: round(2),
            points: Points::Settled(-2),
        },
    ];
    assert_eq!(
        all_round_total(&[round(1), round(2)], &contributions),
        Ok(Points::Settled(10))
    );
    assert_eq!(
        all_round_total(&[round(1), round(2), round(3)], &contributions),
        Ok(Points::Pending { recorded: 10 })
    );
    assert_eq!(
        all_round_total(&[round(1)], &contributions),
        Err(FantasyError::InvalidFacts)
    );
    assert_eq!(
        all_round_total(&[round(1), round(1)], &[]),
        Err(FantasyError::DuplicateIdentity)
    );
    assert_eq!(
        all_round_total(&[round(1)], &[contributions[0], contributions[0]]),
        Err(FantasyError::DuplicateIdentity)
    );
}
#[test]
fn explicit_future_rounds_allow_live_overall_ranks_without_settling() {
    let contributions = [
        RoundContribution {
            round: round(1),
            points: Points::Settled(12),
        },
        RoundContribution {
            round: round(2),
            points: Points::NotStarted,
        },
    ];
    let result = golfer_standings(
        &[round(1), round(2)],
        &[golfer(1)],
        &[SeasonEntry {
            id: golfer(1),
            rounds: &contributions,
        }],
    )
    .unwrap();
    assert_eq!(result[0].points, Points::Provisional(12));
    assert_eq!(result[0].rank, Some(1));
    assert_eq!(Points::NotStarted.total(), None);
}
#[test]
fn golfer_board_includes_unselected_roster_and_ties_are_not_broken_by_identity() {
    let a = [RoundContribution {
        round: round(1),
        points: Points::Settled(12),
    }];
    let b = [RoundContribution {
        round: round(1),
        points: Points::Settled(8),
    }];
    let c = [RoundContribution {
        round: round(1),
        points: Points::Settled(5),
    }];
    let entries = [
        SeasonEntry {
            id: golfer(1),
            rounds: &a,
        },
        SeasonEntry {
            id: golfer(2),
            rounds: &b,
        },
        SeasonEntry {
            id: golfer(3),
            rounds: &b,
        },
        SeasonEntry {
            id: golfer(4),
            rounds: &c,
        },
    ];
    let board = golfer_standings(
        &[round(1)],
        &[golfer(5), golfer(4), golfer(3), golfer(2), golfer(1)],
        &entries,
    )
    .unwrap();
    assert_eq!(
        board.iter().map(|row| row.rank).collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(2), Some(4), None]
    );
    assert_eq!(
        board.last().unwrap().points,
        Points::Pending { recorded: 0 }
    );
    assert_eq!(board[1].id, golfer(2));
    assert_eq!(board[2].id, golfer(3));
}
#[test]
fn withheld_board_suppresses_all_ranks_and_retains_identity_order() {
    let hidden = [RoundContribution {
        round: round(1),
        points: Points::Withheld,
    }];
    let visible = [RoundContribution {
        round: round(1),
        points: Points::Settled(12),
    }];
    let entries = [
        SeasonEntry {
            id: golfer(1),
            rounds: &hidden,
        },
        SeasonEntry {
            id: golfer(2),
            rounds: &visible,
        },
    ];
    let board = golfer_standings(&[round(1)], &[golfer(2), golfer(1)], &entries).unwrap();
    assert_eq!(board[0].id, golfer(1));
    assert!(board.iter().all(|row| row.rank.is_none()));
    assert_eq!(
        all_round_total(&[round(1), round(2)], &hidden),
        Ok(Points::Withheld)
    );
}
#[test]
fn manager_roster_types_are_independent_and_reject_duplicates_or_unknown_entries() {
    let id = ManagerId(Uuid::from_u128(1));
    assert_eq!(
        manager_standings(&[round(1)], &[id, id], &[]),
        Err(FantasyError::DuplicateIdentity)
    );
    assert_eq!(
        manager_standings(&[round(1)], &[], &[SeasonEntry { id, rounds: &[] }]),
        Err(FantasyError::InvalidFacts)
    );
}
