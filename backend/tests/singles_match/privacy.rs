use super::*;
use golf_api::domain::match_play::commands::{Basis, Command, CorrectionKind, Event, Outcome};
#[sqlx::test(migrations = "../migrations")]
async fn hidden_correction_noninterference_and_scoring_scope(pool: PgPool) {
    let (session, m, _) = ready(&pool).await;
    sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden=TRUE WHERE id=$1")
        .bind(id(2))
        .execute(&pool)
        .await
        .unwrap();
    let viewer = fixture::session(&pool, 202, None, "viewer", "viewer-match").await;
    let mut replacement = (1..=9)
        .map(|hole_number| Event::Hole {
            hole_number,
            outcome: Outcome::Halved,
            basis: Basis::AgreedHalve {
                play_begun: true,
                mutual_agreement: true,
            },
        })
        .collect::<Vec<_>>();
    replacement.push(Event::Concession {
        conceding_player_id: id(12),
        communicated: true,
        after_hole: 9,
    });
    submit(
        &pool,
        session,
        m,
        1,
        Command::Correct {
            kind: CorrectionKind::RecordingError,
            reason: "Import agreed record".into(),
            superseded_event_ids: vec![],
            replacement: replacement.clone(),
        },
    )
    .await;
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
    let before = serde_json::to_value(
        match_play::reads::get(&pool, viewer, id(5), m, false)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(before.get("revision").is_none());
    assert!(before.get("accepted_events").is_none());
    assert!(before["finish"].is_null());
    assert!(before["confirmed"].is_null());
    let table_before = serde_json::to_value(
        match_play::reads::table(&pool, viewer, id(2))
            .await
            .unwrap(),
    )
    .unwrap();
    let card = match_play::reads::get(&pool, session, id(5), m, true)
        .await
        .unwrap();
    replacement.pop();
    replacement.push(Event::Concession {
        conceding_player_id: id(11),
        communicated: true,
        after_hole: 9,
    });
    submit(
        &pool,
        session,
        m,
        3,
        Command::Correct {
            kind: CorrectionKind::RecordingError,
            reason: "Fix hidden recording".into(),
            superseded_event_ids: card.accepted_events.unwrap().iter().map(|e| e.id).collect(),
            replacement,
        },
    )
    .await;
    assert_eq!(
        before,
        serde_json::to_value(
            match_play::reads::get(&pool, viewer, id(5), m, false)
                .await
                .unwrap()
        )
        .unwrap()
    );
    assert_eq!(
        table_before,
        serde_json::to_value(
            match_play::reads::table(&pool, viewer, id(2))
                .await
                .unwrap()
        )
        .unwrap()
    );
    assert!(matches!(
        match_play::reads::get(&pool, viewer, id(5), m, true).await,
        Err(match_play::Error::Forbidden)
    ));
    let other = fixture::session(&pool, 203, Some(13), "player", "other-flight").await;
    assert!(matches!(
        match_play::reads::get(&pool, other, id(5), m, true).await,
        Err(match_play::Error::Forbidden)
    ));
}
