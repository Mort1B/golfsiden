use super::*;
fn players() -> [Uuid; 2] {
    [Uuid::from_u128(1), Uuid::from_u128(2)]
}
fn validate(events: Vec<Event>, admin: bool) -> Result<MatchState, &'static str> {
    validate_ledger(
        &events
            .into_iter()
            .map(|event| AcceptedEvent {
                id: Uuid::new_v4(),
                event,
            })
            .collect::<Vec<_>>(),
        players(),
        HandicapAllocation::new(super::super::MatchMode::Net, [0, 0]),
        &(1..=18).collect::<Vec<_>>(),
        admin,
    )
}
#[test]
fn blank_attestations_and_numeric_mismatch_are_rejected() {
    for basis in [
        Basis::Numeric {
            first_gross: 4,
            second_gross: 5,
            agreed: false,
        },
        Basis::AgreedHalve {
            play_begun: false,
            mutual_agreement: true,
        },
        Basis::HoleConcession {
            conceding_player_id: players()[0],
            communicated: false,
        },
        Basis::NextStrokeConcession {
            first_gross: 4,
            second_gross: 4,
            conceding_player_id: Uuid::from_u128(9),
            communicated: true,
            agreed: true,
        },
    ] {
        assert!(
            validate(
                vec![Event::Hole {
                    hole_number: 1,
                    outcome: Outcome::Halved,
                    basis
                }],
                false
            )
            .is_err()
        );
    }
    assert!(
        validate(
            vec![Event::Hole {
                hole_number: 1,
                outcome: Outcome::Second,
                basis: Basis::Numeric {
                    first_gross: 4,
                    second_gross: 5,
                    agreed: true
                }
            }],
            true
        )
        .is_err()
    );
}
#[test]
fn organizer_authority_and_reason_are_mandatory() {
    let ruling = Event::Hole {
        hole_number: 1,
        outcome: Outcome::Halved,
        basis: Basis::OrganizerRuling {
            reason: "Recorded ruling".into(),
        },
    };
    assert!(validate(vec![ruling.clone()], false).is_err());
    assert!(validate(vec![ruling], true).is_ok());
    assert!(
        validate(
            vec![Event::Award {
                winner_player_id: players()[0],
                reason: " ".into(),
                after_hole: 0
            }],
            true
        )
        .is_err()
    );
}
#[test]
fn concession_effective_point_and_visibility_are_contiguous() {
    let event = Event::Concession {
        conceding_player_id: players()[0],
        communicated: true,
        after_hole: 0,
    };
    assert!(
        validate(vec![event.clone()], false)
            .unwrap()
            .finish()
            .is_some()
    );
    assert!(visible_event(&event, 9));
    let hidden = Event::Concession {
        conceding_player_id: players()[0],
        communicated: true,
        after_hole: 9,
    };
    assert!(!visible_event(&hidden, 9));
    assert!(validate(vec![hidden], true).is_err());
}
#[test]
fn unknown_mixed_payloads_and_invalid_revisions_fail_decoding() {
    for command in [
        serde_json::json!({"type":"note","player_id":players()[0],"hole_number":1,"gross_strokes":4,"event":{}}),
        serde_json::json!({"type":"concede","conceding_player_id":players()[0]}),
    ] {
        assert!(serde_json::from_value::<Command>(command).is_err());
    }
    for revision in ["0", "-1", "01", "9223372036854775808"] {
        let v = serde_json::json!({"request_id":Uuid::new_v4(),"expected_revision":revision,"command":{"type":"confirm","result_agreed_or_awarded":true}});
        assert!(serde_json::from_value::<Request>(v).is_err());
    }
}
