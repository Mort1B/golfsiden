mod fixture;
use super::*;
use fixture::{fixture, id};
use std::{hint::black_box, time::Instant};

#[test]
#[ignore = "manual projection-only timing; run release with --ignored --nocapture"]
fn projection_benchmark() {
    for count in [1, 3, 5] {
        let data = fixture(count);
        for _ in 0..50 {
            black_box(project_bundle(black_box(&data), id(999), id(101), false).unwrap());
        }
        let target = data.rounds.last().unwrap().id;
        for _ in 0..50 {
            black_box(project_round(black_box(&data), id(999), id(101), false, target).unwrap());
        }
        let mut full = vec![];
        let mut scoped = vec![];
        for sample in 0..500 {
            for candidate in if sample % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let start = Instant::now();
                if candidate {
                    black_box(
                        project_round(black_box(&data), id(999), id(101), false, target).unwrap(),
                    );
                    scoped.push(start.elapsed().as_nanos());
                } else {
                    black_box(project_bundle(black_box(&data), id(999), id(101), false).unwrap());
                    full.push(start.elapsed().as_nanos());
                }
            }
        }
        full.sort_unstable();
        scoped.sort_unstable();
        println!(
            "rounds={count} full_median_ns={} full_p95_ns={} scoped_median_ns={} scoped_p95_ns={}",
            full[250], full[474], scoped[250], scoped[474]
        );
    }
}
#[test]
fn scoped_round_matches_full_projection_bytes_and_internal_finalization() {
    use crate::domain::{
        fantasy::RulesVersion,
        models::{RoundStatus, ScoringFormat},
        score_visibility::VisibilityMode,
    };
    for rules in [RulesVersion::Legacy, RulesVersion::ExceptionalAce] {
        for (status, locked) in [
            (RoundStatus::Draft, false),
            (RoundStatus::Draft, true),
            (RoundStatus::Open, true),
            (RoundStatus::Completed, true),
            (RoundStatus::Locked, true),
        ] {
            for format in [
                ScoringFormat::IndividualStrokePlay,
                ScoringFormat::SinglesMatchPlay,
            ] {
                for mode in [VisibilityMode::Full, VisibilityMode::FrontNine] {
                    let mut data = fixture(3);
                    data.rules = rules;
                    for r in &mut data.rounds {
                        r.status = status;
                        r.scoring_format = format;
                        if !locked {
                            r.locked_at = None;
                        }
                        data.visibility.get_mut(&r.id).unwrap().mode = mode;
                        for owner in data.owners.get_mut(&r.id).unwrap() {
                            owner.facts.round.status = status;
                            owner.facts.round.scoring_format = format;
                        }
                    }
                    for caller in [id(101), id(102), id(999)] {
                        for finalized in [false, true] {
                            let full = project_bundle(&data, id(999), caller, finalized).unwrap();
                            for expected in full.rounds {
                                let actual = project_round(
                                    &data,
                                    id(999),
                                    caller,
                                    finalized,
                                    expected.round.round_id,
                                )
                                .unwrap();
                                assert_eq!(
                                    serde_json::to_vec(&actual).unwrap(),
                                    serde_json::to_vec(&expected).unwrap()
                                );
                                assert_eq!(actual.finalized, expected.finalized);
                                assert_eq!(actual.finalized, finalized);
                                if !locked {
                                    for manager in &actual.managers {
                                        assert_eq!(
                                            manager.lineup.is_some(),
                                            manager.user_id == caller
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn scoped_round_does_not_project_unrelated_invalid_domain_facts() {
    let mut data = fixture(3);
    let target = data.rounds[0].id;
    let unrelated = data.rounds[1].id;
    let expected = project_round(&data, id(999), id(101), false, target).unwrap();
    data.owners.get_mut(&unrelated).unwrap()[0]
        .facts
        .holes
        .clear();
    assert!(matches!(
        project_bundle(&data, id(999), id(101), false),
        Err(Error::Invalid)
    ));
    let actual = project_round(&data, id(999), id(101), false, target).unwrap();
    assert_eq!(
        serde_json::to_vec(&actual).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
    assert!(matches!(
        project_round(&data, id(999), id(101), false, unrelated),
        Err(Error::Invalid)
    ));
    assert!(matches!(
        project_round(&data, id(999), id(101), false, id(998)),
        Err(Error::Authorization(AuthorizationError::NotFound))
    ));
}
