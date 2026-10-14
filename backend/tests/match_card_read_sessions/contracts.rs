use super::support::*;
use crate::match_fixture as seed;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{
    domain::match_play::commands::{Basis, Command, CorrectionKind, Event, Outcome},
    repositories::match_play,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::Uuid;

async fn parity(
    pool: &PgPool,
    app: &axum::Router,
    admin: Uuid,
    token: &str,
) -> Vec<(String, Value)> {
    let mut result = Vec::new();
    for (scoring, path) in [false, true].into_iter().zip(paths(pool).await) {
        let expected = serde_json::to_value(
            match_play::reads::get(pool, admin, ROUND, match_id(pool).await, scoring)
                .await
                .unwrap(),
        )
        .unwrap();
        let r = get(app, &path, token).await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_private(&r);
        assert_eq!(body(r).await, expected);
        assert_eq!(expected.get("revision").is_some(), scoring);
        assert_eq!(expected.get("accepted_events").is_some(), scoring);
        result.push((path, expected));
    }
    result
}
#[sqlx::test(migrations = "../migrations")]
async fn full_payloads_preserve_accepted_outcomes_notes_and_fixed_handicaps(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    for (revision, command) in [
        (
            1,
            Command::Note {
                player_id: PLAYER,
                hole_number: 1,
                gross_strokes: 4,
            },
        ),
        (
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
        ),
        (
            3,
            Command::Note {
                player_id: PLAYER,
                hole_number: 1,
                gross_strokes: 2,
            },
        ),
    ] {
        submit(&pool, admin, revision, command).await;
    }
    let values = parity(&pool, &app, admin, &token).await;
    assert_eq!(values[0].1["lead"], -1);
    assert_eq!(values[0].1["resolved_holes"], 1);
    assert_eq!(values[0].1["notes"][0]["gross_strokes"], 2);
    assert_eq!(values[0].1["opponents"][1]["playing_handicap"], 40);
    assert_eq!(values[0].1["relative_handicaps"], json!([0, 40]));
    submit(
        &pool,
        admin,
        4,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: seed::id(12),
                communicated: true,
                after_hole: 1,
            },
        },
    )
    .await;
    let values = parity(&pool, &app, admin, &token).await;
    assert_eq!(values[0].1["finish"]["type"], "conceded");
    assert_eq!(values[0].1["confirmed"], false);
    assert!(values[0].1["half_points"].is_null());
    submit(
        &pool,
        admin,
        5,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let values = parity(&pool, &app, admin, &token).await;
    assert_eq!(values[0].1["half_points"], json!([2, 0]));
    let ids = values[1].1["accepted_events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| serde_json::from_value(e["id"].clone()).unwrap())
        .collect();
    submit(
        &pool,
        admin,
        6,
        Command::Correct {
            kind: CorrectionKind::OrganizerRuling,
            reason: "Recorded decision".into(),
            superseded_event_ids: ids,
            replacement: vec![Event::Award {
                winner_player_id: seed::id(12),
                reason: "Organizer award".into(),
                after_hole: 0,
            }],
        },
    )
    .await;
    let values = parity(&pool, &app, admin, &token).await;
    assert_eq!(values[0].1["correction_pending"], true);
    assert_eq!(values[0].1["confirmed"], false);
    assert!(values[0].1["half_points"].is_null());
    submit(
        &pool,
        admin,
        7,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let expected = parity(&pool, &app, admin, &token).await;
    assert_eq!(expected[0].1["half_points"], json!([0, 2]));
    sqlx::query("UPDATE players SET current_handicap_index=1")
        .execute(&pool)
        .await
        .unwrap();
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut reads = Vec::new();
    for (path, value) in expected.into_iter().cycle().take(12) {
        let router = app.clone();
        let t = token.clone();
        reads.push(tokio::spawn(async move {
            let r = get(&router, &path, &t).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_private(&r);
            assert_eq!(body(r).await, value);
        }));
    }
    for read in reads {
        timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn roles_flights_errors_and_mutation_transport_are_preserved(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let match_id = match_id(&pool).await;
    let other: Uuid =
        sqlx::query_scalar("SELECT id FROM singles_matches WHERE round_id=$1 AND id<>$2")
            .bind(ROUND)
            .bind(match_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    for (n, linked, role) in [
        (201, None, "scorer"),
        (202, Some(11), "player"),
        (203, None, "player"),
        (204, Some(13), "viewer"),
    ] {
        let token = format!("match-card-role-{n}");
        seed::session(&pool, n, linked, role, &token).await;
        for target in [match_id, other] {
            for scoring in [false, true] {
                let suffix = if scoring { "/scoring" } else { "" };
                let path = format!("/api/rounds/{ROUND}/match-play/matches/{target}{suffix}");
                let r = get(&app, &path, &token).await;
                assert_private(&r);
                assert_eq!(
                    r.status(),
                    if !scoring
                        || role == "scorer"
                        || (role == "player" && linked.is_some() && target == match_id)
                    {
                        StatusCode::OK
                    } else {
                        StatusCode::FORBIDDEN
                    }
                );
            }
        }
    }
    for suffix in ["", "/scoring"] {
        for round in [ROUND, Uuid::new_v4()] {
            let path = format!(
                "/api/rounds/{round}/match-play/matches/{}{suffix}",
                Uuid::new_v4()
            );
            let r = get(&app, &path, seed::TOKEN).await;
            assert_eq!(r.status(), StatusCode::NOT_FOUND);
            assert_private(&r);
        }
    }
    let path = format!("/api/rounds/invalid/match-play/matches/{match_id}");
    let r = get(&app, &path, seed::TOKEN).await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert_private(&r);
    let r = app
        .oneshot(
            Request::post(format!(
                "/api/rounds/{ROUND}/match-play/matches/{match_id}/commands"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_private(&r);
    assert_eq!(body(r).await["error"]["code"], "unauthenticated");
}

#[sqlx::test(migrations = "../migrations")]
async fn scoring_requires_both_opponents_even_when_one_remains_eligible(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    seed::session(&pool, 211, Some(11), "player", "one-opponent").await;
    let mut tx = pool.begin().await.unwrap();
    // Defensive fixture only: normal open-round writes cannot create this state.
    sqlx::query(
        "ALTER TABLE flight_memberships DISABLE TRIGGER flight_memberships_protect_round_pairing",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("DELETE FROM flight_memberships WHERE round_id=$1 AND player_id=$2")
        .bind(ROUND)
        .bind(seed::id(12))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "ALTER TABLE flight_memberships ENABLE TRIGGER flight_memberships_protect_round_pairing",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    for (index, path) in paths(&pool).await.into_iter().enumerate() {
        let r = get(&app, &path, "one-opponent").await;
        assert_private(&r);
        assert_eq!(
            r.status(),
            if index == 0 {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            }
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn hidden_member_payload_does_not_leak_changed_terminal_result(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, _) = session(&pool).await;
    seed::session(&pool, 220, None, "viewer", "hidden-match-member").await;
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
        conceding_player_id: seed::id(12),
        communicated: true,
        after_hole: 9,
    });
    submit(
        &pool,
        admin,
        1,
        Command::Correct {
            kind: CorrectionKind::RecordingError,
            reason: "Import agreement".into(),
            superseded_event_ids: vec![],
            replacement: replacement.clone(),
        },
    )
    .await;
    submit(
        &pool,
        admin,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let path = &paths(&pool).await[0];
    let baseline = body(get(&app, path, "hidden-match-member").await).await;
    assert_eq!(baseline["holes"].as_array().unwrap().len(), 9);
    assert_eq!(baseline["events"].as_array().unwrap().len(), 9);
    for field in ["finish", "confirmed", "correction_pending", "half_points"] {
        assert!(baseline[field].is_null());
    }
    assert!(baseline.get("revision").is_none());
    assert!(baseline.get("accepted_events").is_none());
    let accepted = match_play::reads::get(&pool, admin, ROUND, match_id(&pool).await, true)
        .await
        .unwrap()
        .accepted_events
        .unwrap();
    replacement.pop();
    replacement.push(Event::Award {
        winner_player_id: PLAYER,
        reason: "Hidden ruling".into(),
        after_hole: 9,
    });
    submit(
        &pool,
        admin,
        3,
        Command::Correct {
            kind: CorrectionKind::OrganizerRuling,
            reason: "Hidden correction".into(),
            superseded_event_ids: accepted.iter().map(|e| e.id).collect(),
            replacement,
        },
    )
    .await;
    submit(
        &pool,
        admin,
        4,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    assert_eq!(
        body(get(&app, path, "hidden-match-member").await).await,
        baseline
    );
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn late_notes_loading_preserves_coherent_aggregate_revision_and_note_snapshot(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    submit(
        &pool,
        admin,
        1,
        Command::Note {
            player_id: PLAYER,
            hole_number: 1,
            gross_strokes: 4,
        },
    )
    .await;
    for path in paths(&pool).await {
        let baseline = body(get(&app, &path, &token).await).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query("LOCK TABLE singles_match_notes IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *gate)
            .await
            .unwrap();
        let router = app.clone();
        let p = path.clone();
        let t = token.clone();
        let read = tokio::spawn(async move { get(&router, &p, &t).await });
        wait_blocked(&pool, pid).await;
        let id = match_id(&pool).await;
        let request = Uuid::new_v4();
        let gross = if baseline["notes"][0]["gross_strokes"] == 7 {
            8i16
        } else {
            7i16
        };
        let command =
            json!({"type":"note","player_id":PLAYER,"hole_number":1,"gross_strokes":gross});
        sqlx::query("SELECT set_config('app.match_actor',$1::text,true),set_config('app.match_session',$2::text,true),set_config('app.score_mutation_round_id',$3::text,true),set_config('app.match_command',$4,true),set_config('app.match_request',$5::text,true),set_config('app.match_note_id',$6::text,true)").bind(USER).bind(admin).bind(ROUND).bind(command.to_string()).bind(request).bind(id).execute(&mut *gate).await.unwrap();
        sqlx::query("UPDATE singles_match_notes SET gross_strokes=CASE WHEN gross_strokes=7 THEN 8 ELSE 7 END WHERE match_id=$1").bind(id).execute(&mut *gate).await.unwrap();
        let revision: i64 = sqlx::query_scalar(
            "UPDATE singles_matches SET revision=revision+1 WHERE id=$1 RETURNING revision",
        )
        .bind(id)
        .fetch_one(&mut *gate)
        .await
        .unwrap();
        sqlx::query("INSERT INTO singles_match_receipts(user_id,request_id,match_id,request_hash,applied_revision) VALUES($1,$2,$3,$4,$5)").bind(USER).bind(request).bind(id).bind(vec![0u8;32]).bind(revision).execute(&mut *gate).await.unwrap();
        gate.commit().await.unwrap();
        let changed = score_state(&pool).await;
        let mut events = state.live_events.subscribe();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        assert_private(&r);
        assert_eq!(body(r).await, baseline);
        let fresh = body(get(&app, &path, &token).await).await;
        assert_ne!(fresh, baseline);
        assert_ne!(fresh["notes"], baseline["notes"]);
        if path.ends_with("/scoring") {
            assert_eq!(fresh["revision"], revision.to_string());
        }
        assert_eq!(score_state(&pool).await, changed);
        assert!(events.try_recv().is_err());
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn cancellation_releases_session_authority_on_both_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    for path in paths(&pool).await {
        let (session, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        read.abort();
        assert!(read.await.unwrap_err().is_cancelled());
        gate.commit().await.unwrap();
        timeout(
            Duration::from_secs(10),
            golf_api::repositories::auth::revoke_session(&pool, session),
        )
        .await
        .unwrap()
        .unwrap();
        let (_, fresh) = super::support::session(&pool).await;
        assert_eq!(get(&app, &path, &fresh).await.status(), StatusCode::OK);
    }
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}
