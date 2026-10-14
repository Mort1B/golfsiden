use super::super::load::{Identity, Loaded, Manager, Round, SelectionRow};
use crate::domain::{
    fantasy::{RulesVersion, SourceToken, projection::*},
    models::{RoundStatus, ScoringFormat},
    score_visibility::{VisibilityMetadata, VisibilityMode},
};
use chrono::{TimeZone, Utc};
use std::collections::BTreeMap;
use uuid::Uuid;

pub(super) fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
pub(super) fn fixture(count: i16) -> Loaded {
    let at = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let golfers: Vec<_> = (1..=36)
        .map(|n| Identity {
            id: id(n),
            display_name: format!("Golfer {n}"),
        })
        .collect();
    let managers: Vec<_> = (101..=124)
        .map(|n| Manager {
            id: id(n),
            display_name: format!("Manager {n}"),
            entered_at: at,
        })
        .collect();
    let mut rounds = vec![];
    let mut owners = BTreeMap::new();
    let mut selections = BTreeMap::new();
    let mut visibility = BTreeMap::new();
    for n in 1..=count {
        let r = id(200 + n as u128);
        rounds.push(Round {
            id: r,
            round_number: n,
            name: format!("Round {n}"),
            status: RoundStatus::Open,
            scoring_format: ScoringFormat::IndividualStrokePlay,
            number_of_holes: 18,
            locked_at: Some(at),
        });
        visibility.insert(
            r,
            VisibilityMetadata {
                mode: VisibilityMode::Full,
            },
        );
        owners.insert(
            r,
            golfers
                .iter()
                .map(|g| {
                    let holes: Vec<_> = (1..=18)
                        .map(|h| Hole {
                            id: id(300 + h as u128),
                            hole_number: h,
                            par: 4,
                            stroke_index: h,
                        })
                        .collect();
                    let scores = holes
                        .iter()
                        .map(|h| Score {
                            hole_id: h.id,
                            gross_strokes: if h.hole_number == 1 {
                                1
                            } else {
                                3 + (g.id.as_u128() % 4) as i16
                            },
                        })
                        .collect();
                    OwnerFacts {
                        facts: Facts {
                            round: crate::domain::fantasy::projection::Round {
                                id: r,
                                scoring_format: ScoringFormat::IndividualStrokePlay,
                                status: RoundStatus::Open,
                                number_of_holes: 18,
                                handicap_enabled: true,
                                handicap_allowance_percent: 100,
                            },
                            owner_id: g.id,
                            players: vec![g.id],
                            holes,
                            snapshots: vec![Snapshot {
                                player_id: g.id,
                                course_handicap: 18,
                                playing_handicap: 18,
                            }],
                            team_snapshot: None,
                            scores,
                            four_ball: vec![],
                            stableford: vec![],
                            confirmation: vec![serde_json::json!({"confirmed":true})],
                            matches: vec![],
                        },
                        source: SourceToken([1; 32]),
                        non_finish: None,
                    }
                })
                .collect(),
        );
        for (index, m) in managers.iter().enumerate() {
            let picks: Vec<_> = (0..4)
                .map(|offset| golfers[(index + offset) % golfers.len()].id)
                .collect();
            selections.insert(
                (r, m.id),
                SelectionRow {
                    round_id: r,
                    user_id: m.id,
                    state: "locked".into(),
                    captain: Some(picks[0]),
                    picks: Some(picks),
                    origin: Some("submitted".into()),
                    source_round: None,
                },
            );
        }
    }
    Loaded {
        rules: RulesVersion::ExceptionalAce,
        rounds,
        golfers,
        managers,
        selections,
        owners,
        visibility,
    }
}
