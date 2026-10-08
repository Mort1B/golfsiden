use super::*;
use crate::domain::fantasy::selections::*;
use std::collections::BTreeSet;
fn lineup(first: u128, captain: u128) -> Lineup {
    Lineup::new(
        &[
            golfer(first),
            golfer(first + 1),
            golfer(first + 2),
            golfer(first + 3),
        ],
        golfer(captain),
    )
    .unwrap()
}
fn eligible() -> BTreeSet<GolferId> {
    (1..=8).map(golfer).collect()
}
fn previous(order: u32, first: u128, captain: u128) -> EarlierLockedLineup {
    EarlierLockedLineup {
        round: round(u128::from(order)),
        round_order: order,
        lineup: lineup(first, captain),
    }
}
fn resolve(
    current: Option<&Lineup>,
    prior: &[EarlierLockedLineup],
    eligible: &BTreeSet<GolferId>,
) -> Result<Selection, FantasyError> {
    resolve_selection(DeadlineSelection {
        round: round(3),
        round_order: 3,
        participating_at_deadline: true,
        eligible_at_deadline: eligible,
        accepted_current: current,
        earlier_locked_at_deadline: prior,
    })
}
#[test]
fn exactly_four_unique_and_captain_among_them() {
    for picks in [
        vec![golfer(1); 4],
        vec![golfer(1), golfer(2), golfer(3)],
        (1..=5).map(golfer).collect(),
    ] {
        assert_eq!(
            Lineup::new(&picks, golfer(1)),
            Err(FantasyError::InvalidLineup)
        );
    }
    assert_eq!(
        Lineup::new(&[golfer(1), golfer(2), golfer(3), golfer(4)], golfer(5)),
        Err(FantasyError::InvalidLineup)
    );
}
#[test]
fn valid_current_beats_previous_and_preserves_accepted_captain() {
    let current = lineup(5, 6);
    assert_eq!(
        resolve(Some(&current), &[previous(1, 1, 2)], &eligible()),
        Ok(Selection::Locked {
            round: round(3),
            lineup: current,
            origin: Origin::Submitted
        })
    );
}
#[test]
fn carries_nearest_locked_lineup_and_captain_in_current_round() {
    let nearest = previous(2, 5, 7);
    assert_eq!(
        resolve(None, &[nearest.clone(), previous(1, 1, 2)], &eligible()),
        Ok(Selection::Locked {
            round: round(3),
            lineup: nearest.lineup,
            origin: Origin::CarriedForward {
                source_round: round(2)
            }
        })
    );
}
#[test]
fn invalid_current_falls_back_and_ineligible_nearest_never_uses_older() {
    let valid: BTreeSet<_> = (1..=4).map(golfer).collect();
    let invalid_current = lineup(5, 6);
    assert_eq!(
        resolve(Some(&invalid_current), &[previous(1, 1, 2)], &valid),
        Ok(Selection::Locked {
            round: round(3),
            lineup: lineup(1, 2),
            origin: Origin::CarriedForward {
                source_round: round(1)
            }
        })
    );
    assert_eq!(
        resolve(None, &[previous(1, 1, 2), previous(2, 5, 6)], &valid),
        Ok(Selection::Invalid)
    );
}
#[test]
fn ineligible_captain_cannot_be_promoted_or_replaced() {
    let mut valid = eligible();
    valid.remove(&golfer(2));
    assert_eq!(
        resolve(None, &[previous(1, 1, 2)], &valid),
        Ok(Selection::Invalid)
    );
}
#[test]
fn no_previous_or_too_few_eligible_have_explicit_missed_invalid_results() {
    assert_eq!(resolve(None, &[], &eligible()), Ok(Selection::Missed));
    assert_eq!(
        resolve(Some(&lineup(1, 2)), &[], &BTreeSet::new()),
        Ok(Selection::Invalid)
    );
}
#[test]
fn future_current_and_duplicate_source_rounds_are_rejected() {
    for order in [3, 4] {
        assert_eq!(
            resolve(None, &[previous(order, 1, 2)], &eligible()),
            Err(FantasyError::InvalidFacts)
        );
    }
    assert_eq!(
        resolve(None, &[previous(1, 1, 2), previous(1, 5, 6)], &eligible()),
        Err(FantasyError::DuplicateIdentity)
    );
}
#[test]
fn nonmembers_and_late_entries_are_not_implicitly_enrolled() {
    assert_eq!(
        resolve_selection(DeadlineSelection {
            round: round(3),
            round_order: 3,
            participating_at_deadline: false,
            eligible_at_deadline: &eligible(),
            accepted_current: None,
            earlier_locked_at_deadline: &[previous(1, 1, 2)]
        }),
        Ok(Selection::NotParticipating)
    );
}
#[test]
fn carried_lineup_can_be_source_again_without_carrying_scores() {
    let first = resolve(None, &[previous(1, 1, 2)], &eligible()).unwrap();
    let Selection::Locked {
        round: source_round,
        lineup,
        ..
    } = first
    else {
        panic!()
    };
    let next = resolve_selection(DeadlineSelection {
        round: round(4),
        round_order: 4,
        participating_at_deadline: true,
        eligible_at_deadline: &eligible(),
        accepted_current: None,
        earlier_locked_at_deadline: &[EarlierLockedLineup {
            round: source_round,
            round_order: 3,
            lineup: lineup.clone(),
        }],
    })
    .unwrap();
    assert_eq!(
        next,
        Selection::Locked {
            round: round(4),
            lineup,
            origin: Origin::CarriedForward {
                source_round: round(3)
            }
        }
    );
}
