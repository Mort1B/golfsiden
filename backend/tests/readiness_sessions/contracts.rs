use super::support::*;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use golf_api::{
    domain::models::TournamentRole,
    repositories::{round_completion, round_lifecycle},
};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::Uuid;

#[sqlx::test(migrations = "../migrations")]
async fn cancellation_releases_session_authority_for_both_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
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
}

async fn snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_object(
      'rounds',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r),
      'trips',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tournaments t),
      'scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scores s),
      'snapshots',(SELECT jsonb_agg(to_jsonb(s) ORDER BY round_id,player_id) FROM round_handicap_snapshots s),
      'confirmations',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scorecard_confirmations s),
      'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY tournament_id,user_id) FROM tournament_memberships m),
      'teams',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM teams t),
      'flights',(SELECT jsonb_agg(to_jsonb(f) ORDER BY id) FROM flights f)
    )::text)").fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_reads_preserve_full_payload_and_blocker_order_without_effects(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let expected_pairing = serde_json::to_value(
        round_lifecycle::pairing_validation(&pool, ROUND)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let mut expected_completion = serde_json::to_value(
        round_completion::validation(&pool, ROUND)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    expected_completion["visibility"] = json!({"mode":"full"});
    assert_eq!(
        expected_pairing["issues"][0]["code"],
        "tournament_not_openable"
    );
    assert_eq!(expected_completion["owners"].as_array().unwrap().len(), 4);
    let before = snapshot(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut reads = Vec::new();
    for path in paths().into_iter().cycle().take(12) {
        let router = app.clone();
        let t = token.clone();
        let expected = if path.ends_with("/pairing-validation") {
            expected_pairing.clone()
        } else {
            expected_completion.clone()
        };
        reads.push(tokio::spawn(async move {
            let r = get(&router, &path, &t).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
            assert_eq!(body(r).await, expected);
        }));
    }
    for read in reads {
        timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(snapshot(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn roles_resource_errors_cache_and_mutation_boundaries_are_preserved(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    sqlx::query("UPDATE users SET role='viewer' WHERE id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    for role in [
        TournamentRole::Admin,
        TournamentRole::Scorer,
        TournamentRole::Player,
        TournamentRole::Viewer,
    ] {
        sqlx::query(
            "UPDATE tournament_memberships SET role=$1 WHERE tournament_id=$2 AND user_id=$3",
        )
        .bind(role)
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        for path in paths() {
            let r = get(&app, &path, &token).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        }
    }
    let _other = empty_round(&pool).await;
    sqlx::query("UPDATE users SET role='admin' WHERE id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1 AND tournament_id=$2")
        .bind(USER)
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    for path in paths() {
        let r = get(&app, &path, &token).await;
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        assert_eq!(body(r).await["error"]["code"], "forbidden");
        let r = get(&app, &path, "unknown").await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    }
    for target in [Uuid::new_v4().to_string(), "invalid".into()] {
        for suffix in ["pairing-validation", "completion-validation"] {
            let r = get(&app, &format!("/api/rounds/{target}/{suffix}"), &token).await;
            assert_eq!(
                r.status(),
                if target == "invalid" {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::NOT_FOUND
                }
            );
            assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        }
    }
    // Adjacent lifecycle POSTs keep their unauthenticated contract and do not
    // inherit the GET-only cache middleware or new retry boundary.
    for suffix in ["open", "complete", "lock"] {
        let r = app
            .clone()
            .oneshot(
                Request::post(format!("/api/rounds/{ROUND}/{suffix}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn empty_and_singles_readiness_keep_existing_envelopes(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let (_, round) = empty_round(&pool).await;
    let pairing = body(
        get(
            &app,
            &format!("/api/rounds/{round}/pairing-validation"),
            &token,
        )
        .await,
    )
    .await;
    assert_eq!(pairing["ready"], false);
    assert!(
        pairing["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "no_active_entrants")
    );
    for key in [
        "missing_players",
        "ineligible_players",
        "team_sizes",
        "missing_flight_players",
        "ineligible_flight_players",
        "flight_sizes",
        "legacy_individual_groups",
        "split_teams",
    ] {
        assert_eq!(pairing[key], json!([]));
    }
    let completion = body(
        get(
            &app,
            &format!("/api/rounds/{round}/completion-validation"),
            &token,
        )
        .await,
    )
    .await;
    assert_eq!(completion["owners"], json!([]));
    assert_eq!(completion["ready_to_complete"], false);
    assert!(
        completion["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "no_required_owners")
    );
    sqlx::query("UPDATE rounds SET scoring_format='singles_match_play' WHERE id=$1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    let r = get(&app, &paths()[1], &token).await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
    let r = get(&app, &paths()[0], &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    let actual = body(r).await;
    let expected = serde_json::to_value(
        round_lifecycle::pairing_validation(&pool, ROUND)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert!(
        actual["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "invalid_match_assignments")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn hidden_final_projection_preserves_exact_role_redaction_and_snapshots(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    open_final(&pool, session).await;
    sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden=true WHERE id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    for role in [
        TournamentRole::Admin,
        TournamentRole::Scorer,
        TournamentRole::Player,
        TournamentRole::Viewer,
    ] {
        sqlx::query(
            "UPDATE tournament_memberships SET role=$1 WHERE tournament_id=$2 AND user_id=$3",
        )
        .bind(role)
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        let r = get(
            &app,
            &format!("/api/rounds/{FINAL}/completion-validation"),
            &token,
        )
        .await;
        assert_eq!(r.status(), StatusCode::OK);
        let v = body(r).await;
        assert_eq!(v["owners"].as_array().unwrap().len(), 8);
        if role == TournamentRole::Admin {
            assert_eq!(v["visibility"]["mode"], "full");
            assert_eq!(v["ready_to_complete"], false);
        } else {
            assert_eq!(v["visibility"]["mode"], "front_nine");
            assert!(v["ready_to_complete"].is_null());
            assert!(v["ready_to_lock"].is_null());
            assert_eq!(v["issues"], json!([]));
            for o in v["owners"].as_array().unwrap() {
                assert!(o["complete"].is_null());
                assert!(o["confirmed"].is_null());
                assert_eq!(o["required_holes"], 9);
            }
        }
    }
    let before = body(
        get(
            &app,
            &format!("/api/rounds/{FINAL}/completion-validation"),
            &token,
        )
        .await,
    )
    .await;
    sqlx::query("UPDATE players SET current_handicap_index=1 WHERE id=$1")
        .bind(FIRST_PLAYER)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        before,
        body(
            get(
                &app,
                &format!("/api/rounds/{FINAL}/completion-validation"),
                &token
            )
            .await
        )
        .await
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn late_loading_keeps_coherent_facts_then_fresh_reads_observe_changes(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    for path in paths() {
        let before = body(get(&app, &path, &token).await).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_loading(&mut gate, &path, false).await;
        let router = app.clone();
        let pending = path.clone();
        let t = token.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &t).await });
        wait_blocked(&pool, pid).await;
        if path.ends_with("/pairing-validation") {
            sqlx::query("UPDATE flights SET name=name||' changed' WHERE round_id=$1")
                .bind(ROUND)
                .execute(&mut *gate)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE teams SET name=name||' changed' WHERE round_id=$1")
                .bind(ROUND)
                .execute(&mut *gate)
                .await
                .unwrap();
        }
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(body(r).await, before);
        assert_ne!(body(get(&app, &path, &token).await).await, before);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn ready_and_transitioned_states_keep_payloads_and_gets_remain_read_only(pool: PgPool) {
    use golf_api::{
        domain::scorecards::ScoreOwner,
        repositories::{scorecards, tournaments},
    };
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    let updated = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    tournaments::start_authorized(&pool, session, TRIP, updated)
        .await
        .unwrap();
    let ready = body(get(&app, &paths()[0], &token).await).await;
    assert_eq!(ready["ready"], true);
    assert_eq!(ready["issues"], json!([]));
    round_lifecycle::open_authorized(&pool, session, ROUND)
        .await
        .unwrap();
    let holes:Vec<Uuid>=sqlx::query_scalar("SELECT h.id FROM holes h JOIN rounds r ON r.tee_id=h.tee_id WHERE r.id=$1 ORDER BY h.hole_number").bind(ROUND).fetch_all(&pool).await.unwrap();
    let teams: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM teams WHERE round_id=$1 ORDER BY id")
        .bind(ROUND)
        .fetch_all(&pool)
        .await
        .unwrap();
    for team in teams {
        let owner = ScoreOwner::Team { id: team };
        for hole in &holes {
            scorecards::save(
                &pool,
                scorecards::SaveScore {
                    round_id: ROUND,
                    hole_id: *hole,
                    owner,
                    gross_strokes: 4,
                    submitted_by: USER,
                },
            )
            .await
            .unwrap();
        }
        scorecards::confirm(&pool, ROUND, owner, USER)
            .await
            .unwrap();
    }
    for stage in ["open", "completed", "locked"] {
        let before = snapshot(&pool).await;
        let mut events = state.live_events.subscribe();
        let r = get(&app, &paths()[1], &token).await;
        assert_eq!(r.status(), StatusCode::OK);
        let v = body(r).await;
        assert_eq!(v["status"], stage);
        assert_eq!(v["ready_to_complete"], stage == "open");
        assert_eq!(v["ready_to_lock"], stage == "completed");
        assert!(
            v["owners"]
                .as_array()
                .unwrap()
                .iter()
                .all(|o| o["confirmed"] == true
                    && o["complete"] == true
                    && o["holes_scored"] == 18)
        );
        let r = get(&app, &paths()[0], &token).await;
        assert_eq!(r.status(), StatusCode::OK);
        let v = body(r).await;
        assert_eq!(v["ready"], false);
        assert_eq!(v["issues"][0]["code"], "round_not_draft");
        assert_eq!(snapshot(&pool).await, before);
        assert!(events.try_recv().is_err());
        match stage {
            "open" => {
                round_completion::complete_authorized(&pool, session, ROUND)
                    .await
                    .unwrap();
            }
            "completed" => {
                round_completion::lock_authorized(&pool, session, ROUND)
                    .await
                    .unwrap();
            }
            _ => {}
        }
    }
}
