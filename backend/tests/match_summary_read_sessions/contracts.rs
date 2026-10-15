use super::support::*;
use axum::{Router, http::StatusCode};
use golf_api::{
    domain::match_play::commands::{Command, Event},
    repositories::{match_play, round_completion},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use uuid::Uuid;
async fn parity(pool: &PgPool, app: &Router, admin: Uuid, token: &str) -> Vec<Value> {
    let expected = vec![
        serde_json::to_value(
            match_play::completion::get(pool, admin, ROUND)
                .await
                .unwrap(),
        )
        .unwrap(),
        serde_json::to_value(match_play::reads::table(pool, admin, TRIP).await.unwrap()).unwrap(),
    ];
    for (path, value) in paths().into_iter().zip(&expected) {
        let r = get(app, &path, token).await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_private(&r);
        assert_eq!(&body(r).await, value);
    }
    expected
}
#[sqlx::test(migrations = "../migrations")]
async fn payloads_keep_confirmed_units_ties_unranked_order_and_concurrent_read_purity(
    pool: PgPool,
) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    let initial = parity(&pool, &app, admin, &token).await;
    assert_eq!(initial[0]["ready_to_complete"], false);
    assert!(
        initial[1]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["played"] == 0 && e["position"].is_null())
    );
    submit(
        &pool,
        admin,
        1,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: Uuid::from_u128(12),
                communicated: true,
                after_hole: 0,
            },
        },
    )
    .await;
    let pending = parity(&pool, &app, admin, &token).await;
    assert_eq!(pending[0]["ready_to_complete"], false);
    assert!(
        pending[0]["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["terminal"] == true && m["confirmed"] == false)
    );
    assert_eq!(pending[1], initial[1]);
    submit(
        &pool,
        admin,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    finish_second_draw(&pool, admin).await;
    let expected = parity(&pool, &app, admin, &token).await;
    assert_eq!(expected[0]["ready_to_complete"], true);
    assert_eq!(expected[0]["ready_to_lock"], false);
    let ids = expected[0]["matches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["match_id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(ids.windows(2).all(|a| a[0] < a[1]));
    let entries = expected[1]["entries"].as_array().unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|e| e["player_id"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!(PLAYER),
            json!(Uuid::from_u128(13)),
            json!(Uuid::from_u128(14)),
            json!(Uuid::from_u128(12))
        ]
    );
    assert_eq!(
        entries
            .iter()
            .map(|e| e["position"].clone())
            .collect::<Vec<_>>(),
        vec![json!(1), json!(2), json!(2), json!(4)]
    );
    assert_eq!(
        entries
            .iter()
            .map(|e| e["half_points"].clone())
            .collect::<Vec<_>>(),
        vec![json!(2), json!(1), json!(1), json!(0)]
    );
    assert_eq!(entries[0]["wins"], 1);
    assert_eq!(entries[1]["draws"], 1);
    assert_eq!(entries[3]["losses"], 1);
    sqlx::query("UPDATE players SET current_handicap_index=1")
        .execute(&pool)
        .await
        .unwrap();
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut reads = Vec::new();
    for (path, value) in paths().into_iter().zip(expected).cycle().take(12) {
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
    round_completion::complete_authorized(&pool, admin, ROUND)
        .await
        .unwrap();
    let completed = parity(&pool, &app, admin, &token).await;
    assert_eq!(completed[0]["status"], "completed");
    assert_eq!(completed[0]["ready_to_complete"], false);
    assert_eq!(completed[0]["ready_to_lock"], true);
    round_completion::lock_authorized(&pool, admin, ROUND)
        .await
        .unwrap();
    let locked = parity(&pool, &app, admin, &token).await;
    assert_eq!(locked[0]["status"], "locked");
    assert_eq!(locked[0]["ready_to_complete"], false);
    assert_eq!(locked[0]["ready_to_lock"], false);
    assert_eq!(locked[1], completed[1]);
}

#[sqlx::test(migrations = "../migrations")]
async fn roles_hidden_awards_and_exact_membership_errors_are_preserved(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    populate(&pool, admin).await;
    for role in ["admin", "scorer", "player", "viewer"] {
        sqlx::query("UPDATE tournament_memberships SET role=$1::text::tournament_role WHERE tournament_id=$2 AND user_id=$3").bind(role).bind(TRIP).bind(USER).execute(&pool).await.unwrap();
        let v = parity(&pool, &app, admin, &token).await;
        if role == "admin" {
            assert_eq!(v[0]["ready_to_complete"], true);
            assert_eq!(v[1]["entries"][0]["half_points"], 2);
        } else {
            for m in v[0]["matches"].as_array().unwrap() {
                assert!(m["terminal"].is_null());
                assert!(m["confirmed"].is_null());
            }
            assert!(v[0]["ready_to_complete"].is_null());
            assert!(v[0]["ready_to_lock"].is_null());
            assert!(
                v[1]["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|e| e["played"] == 0 && e["position"].is_null())
            );
        }
    }
    // USER is a global admin; missing exact membership still denies both reads.
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    for path in paths() {
        let r = get(&app, &path, &token).await;
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], "forbidden");
    }
    for path in [
        format!("/api/rounds/{}/match-play/completion", Uuid::new_v4()),
        format!("/api/tournaments/{}/match-table", Uuid::new_v4()),
    ] {
        let r = get(&app, &path, &token).await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_private(&r);
    }
    for path in [
        "/api/rounds/invalid/match-play/completion",
        "/api/tournaments/invalid/match-table",
    ] {
        let r = get(&app, path, &token).await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        assert_private(&r);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn empty_draft_completion_and_empty_table_keep_typed_envelopes(pool: PgPool) {
    let state = empty_fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    let v = parity(&pool, &app, admin, &token).await;
    assert_eq!(
        v[0],
        json!({"format":"singles_match_play","round_id":ROUND,"status":"draft","matches":[],"ready_to_complete":false,"ready_to_lock":false})
    );
    assert_eq!(v[1], json!({"tournament_id":TRIP,"entries":[]}));
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    parity(&pool, &app, admin, &token).await;
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn wrong_completion_format_stays_not_found_before_membership(pool: PgPool) {
    crate::match_fixture::draft(&pool, "individual_stroke_play").await;
    let app = golf_api::api::router(golf_api::AppState::new(pool.clone()));
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    let r = get(&app, &paths()[0], crate::match_fixture::TOKEN).await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_private(&r);
}

#[sqlx::test(migrations = "../migrations")]
async fn cancellation_releases_session_authority_on_both_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    for path in paths() {
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

async fn coherent_final_load(pool: PgPool, index: usize) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    submit(
        &pool,
        admin,
        1,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: Uuid::from_u128(12),
                communicated: true,
                after_hole: 0,
            },
        },
    )
    .await;
    let id = match_id(&pool).await;
    let path = &paths()[index];
    let baseline = body(get(&app, path, &token).await).await;
    let mut gate = pool.begin().await.unwrap();
    let pid = blocker(&mut gate).await;
    sqlx::query("LOCK TABLE singles_matches IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let router = app.clone();
    let p = path.clone();
    let t = token.clone();
    let read = tokio::spawn(async move { get(&router, &p, &t).await });
    wait_blocked(&pool, pid).await;
    // Commit an authorized confirmation with its revision, audit and receipt.
    let request = Uuid::new_v4();
    let command = json!({"type":"confirm","result_agreed_or_awarded":true});
    sqlx::query("SELECT set_config('app.match_actor',$1::text,true),set_config('app.match_session',$2::text,true),set_config('app.score_mutation_round_id',$3::text,true),set_config('app.match_command',$4,true),set_config('app.match_request',$5::text,true)").bind(USER).bind(admin).bind(ROUND).bind(command.to_string()).bind(request).execute(&mut *gate).await.unwrap();
    sqlx::query("UPDATE singles_matches SET revision=revision+1,confirmed=TRUE,correction_pending=FALSE,first_half_points=2,second_half_points=0 WHERE id=$1").bind(id).execute(&mut *gate).await.unwrap();
    sqlx::query("INSERT INTO singles_match_receipts(user_id,request_id,match_id,request_hash,applied_revision) VALUES($1,$2,$3,$4,3)").bind(USER).bind(request).bind(id).bind(vec![0u8;32]).execute(&mut *gate).await.unwrap();
    sqlx::query("UPDATE players SET display_name='Changed after snapshot' WHERE id=$1")
        .bind(PLAYER)
        .execute(&mut *gate)
        .await
        .unwrap();
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
    let fresh = body(get(&app, path, &token).await).await;
    assert_ne!(fresh, baseline);
    if index == 0 {
        assert!(
            fresh["matches"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["confirmed"] == true)
        );
    } else {
        assert_eq!(
            fresh["entries"][0]["display_name"],
            "Changed after snapshot"
        );
        assert_eq!(fresh["entries"][0]["half_points"], 2);
    }
    assert_eq!(score_state(&pool).await, changed);
    assert!(events.try_recv().is_err());
}
#[sqlx::test(migrations = "../migrations")]
async fn completion_final_loader_keeps_context_and_progress_in_one_snapshot(pool: PgPool) {
    coherent_final_load(pool, 0).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn table_final_loader_keeps_participants_and_awards_in_one_snapshot(pool: PgPool) {
    coherent_final_load(pool, 1).await;
}
