use super::*;
use crate::domain::fantasy::{holes::*, rounds::*, selections::*, totals::manager_round};

#[test]
fn versions_dispatch_ace_precedence_and_all_other_categories() {
    let snapshot = HoleSnapshot {
        par: 5,
        playing_handicap: 18,
        stroke_index: 1,
        number_of_holes: 18,
    };
    for (gross, category, old, new) in [
        (1, Category::Ace, 10, 20),
        (2, Category::AlbatrossOrBetter, 10, 5),
        (3, Category::AlbatrossOrBetter, 10, 5),
        (4, Category::Eagle, 3, 3),
        (5, Category::Birdie, 1, 1),
        (6, Category::Par, 0, 0),
        (7, Category::Bogey, -1, -1),
        (8, Category::DoubleBogey, -2, -2),
        (9, Category::TripleBogey, -3, -3),
        (10, Category::QuadrupleOrWorse, -5, -5),
    ] {
        assert_eq!(
            score_hole(HoleInput::Strokes { gross, snapshot }).unwrap(),
            HolePoints::Recorded(category)
        );
        assert_eq!(category.points(RulesVersion::Legacy), old);
        assert_eq!(category.points(RulesVersion::ExceptionalAce), new);
    }
    assert_eq!(Category::Pickup.points(RulesVersion::ExceptionalAce), -5);
    assert!(RulesVersion::try_from(0).is_err());
    assert!(RulesVersion::try_from(3).is_err());
    assert_eq!(
        serde_json::to_value(RulesVersion::ExceptionalAce).unwrap(),
        2
    );
    // Net one from three physical strokes is no ace.
    assert_eq!(
        score_hole(HoleInput::Strokes {
            gross: 3,
            snapshot: HoleSnapshot {
                par: 3,
                playing_handicap: 36,
                ..snapshot
            }
        })
        .unwrap(),
        HolePoints::Recorded(Category::Eagle)
    );
}

#[test]
fn shared_team_captain_and_balance_are_explicit_for_both_rules() {
    let ace = [HolePoints::Recorded(Category::Ace)];
    let selection = Selection::Locked {
        round: round(1),
        lineup: Lineup::new(&[golfer(1), golfer(2), golfer(3), golfer(4)], golfer(1)).unwrap(),
        origin: Origin::Submitted,
    };
    for (rules, expected) in [
        (RulesVersion::Legacy, 10),
        (RulesVersion::ExceptionalAce, 20),
    ] {
        let owner = NonMatchOwner {
            owner: Owner::Team {
                team_id: Uuid::from_u128(10),
                partners: [golfer(1), golfer(2)],
            },
            holes: &ace,
            source: token(1),
            finish: Finish::NonFinish { attested: token(1) },
            ranking: None,
        };
        let mut scores = score_non_match_round(rules, 1, 1, &[owner]).unwrap();
        assert_eq!(scores[0].points, Points::Settled(expected));
        assert_eq!(scores[1].points, Points::Settled(expected));
        assert_eq!(
            scores
                .iter()
                .map(|s| s.points.total().unwrap())
                .sum::<i64>(),
            2 * expected
        );
        scores.extend([3, 4].map(|n| GolferPoints {
            golfer: golfer(n),
            points: Points::Settled(0),
        }));
        assert_eq!(
            manager_round(round(1), &selection, &scores).unwrap(),
            Some(Points::Settled(3 * expected))
        );
    }
    let rules = RulesVersion::ExceptionalAce;
    assert_eq!(
        Category::Ace.points(rules),
        4 * Category::AlbatrossOrBetter.points(rules)
    );
    assert_eq!(
        Category::Ace.points(rules),
        20 * Category::Birdie.points(rules)
    );
    assert!(
        Category::Ace.points(rules)
            > 5 * Category::Birdie.points(rules) + placement_points(1).unwrap()
    );
    assert_eq!(
        Points::Settled(Category::AlbatrossOrBetter.points(rules)).multiply(2),
        Ok(Points::Settled(10))
    );
    assert_eq!(
        Points::Settled(Category::Ace.points(rules)).multiply(2),
        Ok(Points::Settled(40))
    );
}
