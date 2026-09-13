use super::*;
use golf_api::domain::match_play::commands::{Basis, CorrectionKind, Event, Outcome};
use golf_api::repositories::match_play::Error;
fn concession(player: u128, after_hole: u8) -> Command {
    Command::Report {
        event: Event::Concession {
            conceding_player_id: id(player),
            communicated: true,
            after_hole,
        },
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn terminal_confirmation_receipts_correction_and_round_lifecycle(pool: PgPool) {
    let (session, m, other) = ready(&pool).await;
    let original = request(1, concession(12, 0));
    let ack = match_play::execute(&pool, session, id(5), m, original.clone())
        .await
        .unwrap()
        .value;
    assert_eq!(ack.applied_revision.as_i64(), 2);
    let table = match_play::reads::table(&pool, session, id(2))
        .await
        .unwrap();
    assert!(table.entries.iter().all(|p| p.played == 0));
    submit(
        &pool,
        session,
        m,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let card = match_play::reads::get(&pool, session, id(5), m, true)
        .await
        .unwrap();
    assert_eq!(card.half_points, Some([2, 0]));
    assert!(card.notes.is_empty());
    assert!(matches!(
        match_play::execute(
            &pool,
            session,
            id(5),
            m,
            request(
                3,
                Command::Note {
                    player_id: id(11),
                    hole_number: 1,
                    gross_strokes: 4
                }
            )
        )
        .await,
        Err(Error::Conflict("match_terminal"))
    ));
    submit(&pool, session, other, 1, concession(14, 0)).await;
    submit(
        &pool,
        session,
        other,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    golf_api::repositories::round_completion::complete_authorized(&pool, session, id(5))
        .await
        .unwrap();
    golf_api::repositories::round_completion::lock_authorized(&pool, session, id(5))
        .await
        .unwrap();
    let replay = match_play::execute(&pool, session, id(5), m, original.clone())
        .await
        .unwrap();
    assert!(!replay.changed);
    assert_eq!(replay.value.applied_revision.as_i64(), 2);
    let mut mismatch = original;
    mismatch.command = concession(11, 0);
    assert!(matches!(
        match_play::execute(&pool, session, id(5), m, mismatch).await,
        Err(Error::Conflict("match_request_mismatch"))
    ));
    let superseded = card
        .accepted_events
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    submit(
        &pool,
        session,
        m,
        3,
        Command::Correct {
            kind: CorrectionKind::RecordingError,
            reason: "Wrong conceder was recorded".into(),
            superseded_event_ids: superseded,
            replacement: vec![],
        },
    )
    .await;
    let reopened = match_play::reads::get(&pool, session, id(5), m, true)
        .await
        .unwrap();
    assert!(reopened.finish.is_none());
    assert_eq!(reopened.confirmed, Some(false));
    assert_eq!(reopened.half_points, None);
    assert!(matches!(
        match_play::execute(&pool, session, id(5), m, request(4, concession(11, 0))).await,
        Err(Error::Conflict("match_round_not_editable"))
    ));
    submit(
        &pool,
        session,
        m,
        4,
        Command::Correct {
            kind: CorrectionKind::OrganizerRuling,
            reason: "Organizer establishes permitted result".into(),
            superseded_event_ids: vec![],
            replacement: vec![Event::Award {
                winner_player_id: id(12),
                reason: "Recorded decision".into(),
                after_hole: 0,
            }],
        },
    )
    .await;
    submit(
        &pool,
        session,
        m,
        5,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let corrected = match_play::reads::get(&pool, session, id(5), m, true)
        .await
        .unwrap();
    assert_eq!(corrected.half_points, Some([0, 2]));
    assert_eq!(
        corrected.round_status,
        golf_api::domain::models::RoundStatus::Locked
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn numeric_notes_preserve_agreement_and_reject_stale_and_forged_provenance(pool: PgPool) {
    let (session, m, _) = ready(&pool).await;
    submit(
        &pool,
        session,
        m,
        1,
        Command::Note {
            player_id: id(11),
            hole_number: 1,
            gross_strokes: 4,
        },
    )
    .await;
    submit(
        &pool,
        session,
        m,
        2,
        Command::Report {
            event: Event::Hole {
                hole_number: 1,
                outcome: Outcome::Second,
                basis: Basis::Numeric {
                    first_gross: 4,
                    second_gross: 4,
                    agreed: true,
                },
            },
        },
    )
    .await;
    submit(
        &pool,
        session,
        m,
        3,
        Command::Note {
            player_id: id(11),
            hole_number: 1,
            gross_strokes: 2,
        },
    )
    .await;
    let card = match_play::reads::get(&pool, session, id(5), m, true)
        .await
        .unwrap();
    assert_eq!(card.lead, -1);
    assert_eq!(card.resolved_holes, 1);
    assert!(matches!(
        match_play::execute(
            &pool,
            session,
            id(5),
            m,
            request(
                3,
                Command::ClearNote {
                    player_id: id(11),
                    hole_number: 1
                }
            )
        )
        .await,
        Err(Error::Conflict("match_revision_conflict"))
    ));
    assert!(
        match_play::execute(&pool, session, id(5), m, request(4, concession(12, 9)))
            .await
            .is_err()
    );
    let before =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_audits WHERE match_id=$1")
            .bind(m)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, 3);
    let player = fixture::session(&pool, 201, Some(11), "player", "match-player").await;
    assert!(matches!(
        match_play::execute(
            &pool,
            player,
            id(5),
            m,
            request(
                4,
                Command::Report {
                    event: Event::Award {
                        winner_player_id: id(11),
                        reason: "Not an admin".into(),
                        after_hole: 1
                    }
                }
            )
        )
        .await,
        Err(Error::Forbidden)
    ));
    submit(
        &pool,
        session,
        m,
        4,
        Command::Report {
            event: Event::Hole {
                hole_number: 2,
                outcome: Outcome::Halved,
                basis: Basis::OrganizerRuling {
                    reason: "Organizer decision".into(),
                },
            },
        },
    )
    .await;
    submit(&pool, player, m, 5, concession(12, 2)).await;
    submit(
        &pool,
        player,
        m,
        6,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
}
