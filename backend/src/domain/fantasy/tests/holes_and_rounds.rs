use super::*;
use crate::domain::fantasy::{holes::*, rounds::*};
fn strokes(gross: i32, par: i32, handicap: i32) -> HoleInput {
    HoleInput::Strokes {
        gross,
        snapshot: HoleSnapshot {
            par,
            playing_handicap: handicap,
            stroke_index: 1,
            number_of_holes: 18,
        },
    }
}
fn recorded(category: Category) -> HolePoints {
    HolePoints::Recorded(category)
}
fn owner<'a>(id: u128, holes: &'a [HolePoints], ranking: i64) -> NonMatchOwner<'a> {
    NonMatchOwner {
        owner: Owner::Individual(golfer(id)),
        holes,
        source: token(1),
        finish: Finish::Complete { confirmed: true },
        ranking: Some(NetRanking::Strokes(ranking)),
    }
}
#[test]
fn all_hole_categories_and_extremes_are_signed() {
    for (gross, expected) in [
        (2, 10),
        (3, 3),
        (4, 1),
        (5, 0),
        (6, -1),
        (7, -2),
        (8, -3),
        (9, -5),
        (13, -5),
    ] {
        let HolePoints::Recorded(category) = score_hole(strokes(gross, 5, 0)).unwrap() else {
            panic!()
        };
        assert_eq!(category.points(), expected);
    }
    assert_eq!(
        score_hole(strokes(5, 4, 36)),
        Ok(recorded(Category::Birdie))
    );
    assert_eq!(score_hole(strokes(3, 3, 36)), Ok(recorded(Category::Eagle)));
    assert_eq!(score_hole(strokes(1, 4, 36)), Ok(recorded(Category::Ace)));
    // A scratch ace still wins over a worse net category on a plus handicap.
    assert_eq!(score_hole(strokes(1, 3, -72)), Ok(recorded(Category::Ace)));
    assert_eq!(
        score_hole(strokes(i32::MAX, 1, i32::MIN)),
        Ok(recorded(Category::QuadrupleOrWorse))
    );
}
#[test]
fn signed_handicap_allocates_given_strokes_to_easiest_holes() {
    let snapshot = HoleSnapshot {
        par: 4,
        playing_handicap: -1,
        stroke_index: 18,
        number_of_holes: 18,
    };
    assert_eq!(
        score_hole(HoleInput::Strokes { gross: 4, snapshot }),
        Ok(recorded(Category::Bogey))
    );
    assert_eq!(score_hole(strokes(4, 4, -1)), Ok(recorded(Category::Par)));
}
#[test]
fn pickup_missing_and_hidden_are_distinct() {
    assert_eq!(
        score_hole(HoleInput::Pickup),
        Ok(recorded(Category::Pickup))
    );
    assert_eq!(Category::Pickup.points(), -5);
    assert_eq!(score_hole(HoleInput::Pending), Ok(HolePoints::Pending));
    assert_eq!(score_hole(HoleInput::Withheld), Ok(HolePoints::Withheld));
}
#[test]
fn invalid_numeric_facts_fail_without_panicking() {
    assert_eq!(
        score_hole(strokes(0, 4, 0)),
        Err(FantasyError::InvalidFacts)
    );
    assert_eq!(
        score_hole(strokes(4, 0, 0)),
        Err(FantasyError::InvalidFacts)
    );
    for (count, index) in [(0, 1), (18, 0), (18, 19)] {
        assert_eq!(
            score_hole(HoleInput::Strokes {
                gross: 4,
                snapshot: HoleSnapshot {
                    par: 4,
                    playing_handicap: 0,
                    stroke_index: index,
                    number_of_holes: count
                }
            }),
            Err(FantasyError::InvalidFacts)
        );
    }
}
#[test]
fn four_ball_ace_from_non_counting_partner_and_pickup_rules() {
    assert_eq!(
        score_four_ball([strokes(1, 4, -72), strokes(3, 4, 36)]),
        Ok(recorded(Category::Ace))
    );
    assert_eq!(
        score_four_ball([HoleInput::Pickup, strokes(4, 4, 0)]),
        Ok(recorded(Category::Par))
    );
    assert_eq!(
        score_four_ball([HoleInput::Pickup, HoleInput::Pickup]),
        Ok(recorded(Category::Pickup))
    );
    assert_eq!(
        score_four_ball([strokes(3, 4, 0), HoleInput::Pending]),
        Ok(HolePoints::Pending)
    );
    assert_eq!(
        score_four_ball([HoleInput::Pickup, HoleInput::Pending]),
        Ok(HolePoints::Pending)
    );
    assert_eq!(
        score_four_ball([HoleInput::Withheld, strokes(0, 0, 0)]),
        Ok(HolePoints::Withheld)
    );
    assert_eq!(
        score_four_ball([strokes(3, 4, 0), strokes(3, 5, 0)]),
        Err(FantasyError::InvalidFacts)
    );
}
#[test]
fn shared_competition_placement_uses_owner_field_once() {
    let holes = [recorded(Category::Par)];
    let scores = [70, 71, 71, 72, 73, 74, 75, 76, 77];
    let owners: Vec<_> = scores
        .iter()
        .enumerate()
        .map(|(index, score)| NonMatchOwner {
            owner: Owner::Team {
                team_id: Uuid::from_u128(100 + index as u128),
                partners: [golfer(1 + 2 * index as u128), golfer(2 + 2 * index as u128)],
            },
            ..owner(1, &holes, *score)
        })
        .collect();
    let result = score_non_match_round(9, 1, &owners).unwrap();
    assert_eq!(result.len(), 18);
    let expected = [10, 8, 8, 5, 4, 3, 2, 1, 0];
    for (index, pair) in result.chunks(2).enumerate() {
        assert_eq!(pair[0].points, Points::Settled(expected[index]));
        assert_eq!(pair[0].points, pair[1].points);
    }
    assert_eq!(placement_points(0), Err(FantasyError::InvalidFacts));
    assert_eq!(placement_points(usize::MAX), Ok(0));
}
#[test]
fn smaller_and_larger_fields_keep_fixed_scale() {
    let holes = [recorded(Category::Par)];
    for count in [1, 3, 27] {
        let owners: Vec<_> = (0..count)
            .map(|index| owner(index as u128 + 1, &holes, index as i64))
            .collect();
        let result = score_non_match_round(count, 1, &owners).unwrap();
        assert_eq!(result.first().unwrap().points, Points::Settled(10));
        assert_eq!(
            result.last().unwrap().points,
            Points::Settled(placement_points(count).unwrap())
        );
    }
}
#[test]
fn stableford_ranks_native_points_descending_and_rejects_mixed_units() {
    let holes = [recorded(Category::Pickup)];
    let owners = [
        NonMatchOwner {
            ranking: Some(NetRanking::Stableford(36)),
            ..owner(1, &holes, 0)
        },
        NonMatchOwner {
            ranking: Some(NetRanking::Stableford(40)),
            ..owner(2, &holes, 0)
        },
    ];
    let result = score_non_match_round(2, 1, &owners).unwrap();
    assert_eq!(result[0].points, Points::Settled(3));
    assert_eq!(result[1].points, Points::Settled(5));
    let mixed = [
        NonMatchOwner {
            ranking: Some(NetRanking::Stableford(36)),
            ..owner(1, &holes, 0)
        },
        owner(2, &holes, 70),
    ];
    assert_eq!(
        score_non_match_round(2, 1, &mixed),
        Err(FantasyError::InvalidFacts)
    );
}
#[test]
fn duplicate_golfer_team_and_partner_ownership_is_rejected() {
    let holes = [recorded(Category::Par)];
    let duplicate = [owner(1, &holes, 1), owner(1, &holes, 2)];
    assert_eq!(
        score_non_match_round(2, 1, &duplicate),
        Err(FantasyError::DuplicateIdentity)
    );
    let duplicate_partner = [NonMatchOwner {
        owner: Owner::Team {
            team_id: Uuid::from_u128(1),
            partners: [golfer(1), golfer(1)],
        },
        ..owner(1, &holes, 1)
    }];
    assert_eq!(
        score_non_match_round(1, 1, &duplicate_partner),
        Err(FantasyError::DuplicateIdentity)
    );
    let duplicate_team: Vec<_> = [1, 3]
        .iter()
        .map(|id| NonMatchOwner {
            owner: Owner::Team {
                team_id: Uuid::from_u128(1),
                partners: [golfer(*id), golfer(id + 1)],
            },
            ..owner(*id, &holes, 1)
        })
        .collect();
    assert_eq!(
        score_non_match_round(2, 1, &duplicate_team),
        Err(FantasyError::DuplicateIdentity)
    );
}
#[test]
fn nonfinish_preserves_recorded_points_excludes_placement_and_omits_unplayed() {
    let holes = [
        recorded(Category::Birdie),
        recorded(Category::TripleBogey),
        recorded(Category::Pickup),
        HolePoints::Pending,
    ];
    let complete = [recorded(Category::Par); 4];
    let owners = [
        NonMatchOwner {
            finish: Finish::NonFinish { attested: token(1) },
            ..owner(1, &holes, 0)
        },
        owner(2, &complete, 80),
    ];
    let result = score_non_match_round(2, 4, &owners).unwrap();
    assert_eq!(result[0].points, Points::Settled(-7));
    assert_eq!(result[1].points, Points::Settled(10));
}
#[test]
fn stale_disposition_returns_pending_and_keeps_new_recorded_points() {
    let holes = [recorded(Category::Eagle), HolePoints::Pending];
    let owners = [NonMatchOwner {
        finish: Finish::NonFinish { attested: token(0) },
        ..owner(1, &holes, 0)
    }];
    assert_eq!(
        score_non_match_round(1, 2, &owners).unwrap()[0].points,
        Points::Pending { recorded: 3 }
    );
    let missing = [HolePoints::Pending; 2];
    let settled = [NonMatchOwner {
        finish: Finish::NonFinish { attested: token(1) },
        ..owner(1, &missing, 0)
    }];
    assert_eq!(
        score_non_match_round(1, 2, &settled).unwrap()[0].points,
        Points::Settled(0)
    );
}
#[test]
fn incomplete_fields_and_unconfirmed_cards_cannot_finalize_placement() {
    let holes = [recorded(Category::Par)];
    for finish in [Finish::Playing, Finish::Complete { confirmed: false }] {
        let owners = [
            owner(1, &holes, 70),
            NonMatchOwner {
                finish,
                ..owner(2, &holes, 80)
            },
        ];
        assert_eq!(
            score_non_match_round(2, 1, &owners).unwrap()[0].points,
            Points::Provisional(10)
        );
    }
    let missing = [HolePoints::Pending];
    assert_eq!(
        score_non_match_round(1, 1, &[owner(1, &missing, 0)]),
        Err(FantasyError::InvalidFacts)
    );
    assert_eq!(
        score_non_match_round(2, 1, &[owner(1, &holes, 0)]),
        Err(FantasyError::InvalidFacts)
    );
    assert_eq!(
        score_non_match_round(1, 2, &[owner(1, &holes, 0)]),
        Err(FantasyError::InvalidFacts)
    );
}
#[test]
fn concealed_holes_do_not_leak_placement_or_finality() {
    let visible = [recorded(Category::Par)];
    let concealed = [HolePoints::Withheld];
    let mut outputs = Vec::new();
    for finish in [
        Finish::Playing,
        Finish::Complete { confirmed: true },
        Finish::NonFinish { attested: token(0) },
    ] {
        let owners = [
            owner(1, &visible, 70),
            NonMatchOwner {
                finish,
                ..owner(2, &concealed, i64::MAX)
            },
        ];
        outputs.push(score_non_match_round(2, 1, &owners).unwrap());
    }
    assert!(outputs.iter().all(|output| *output == outputs[0]));
    assert!(
        outputs[0]
            .iter()
            .all(|entry| entry.points == Points::Withheld)
    );
}
