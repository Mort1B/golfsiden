use super::support::*;
use crate::stableford_fixture as seed;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{
    domain::{
        scorecards::ExpectedScore,
        stableford::card::{Input, read_projection},
    },
    repositories::stableford,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::Uuid;

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_full_payloads_preserve_native_points_actual_strokes_and_frozen_handicaps(
    pool: PgPool,
) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    for player in [11, 13] {
        for hole in 1..=18 {
            seed::numeric(
                &pool,
                admin,
                player,
                hole,
                if player == 11 && hole == 1 { 20 } else { 4 },
            )
            .await;
        }
    }
    seed::numeric(&pool, admin, 12, 1, 4).await;
    stableford::save_conditional(
        &pool,
        seed::operation(admin, 12, 2, Input::NoScore {}, ExpectedScore::Absent {}),
    )
    .await
    .unwrap();
    stableford::confirm(&pool, admin, ROUND, PLAYER)
        .await
        .unwrap();
    let mut expected = Vec::new();
    for player in [PLAYER, seed::id(12), seed::id(13)] {
        for scoring in [false, true] {
            let card = stableford::get(&pool, admin, ROUND, player, scoring)
                .await
                .unwrap();
            let value = if scoring {
                serde_json::to_value(card)
            } else {
                serde_json::to_value(read_projection(card))
            }
            .unwrap();
            if player == PLAYER {
                assert_eq!(value["confirmed"], true);
                assert_eq!(value["holes"][0]["score"]["input"]["gross_strokes"], 20);
                assert_eq!(value["holes"][0]["gross_points"], 0);
                assert_eq!(
                    value["values"],
                    json!({"gross_points":34,"net_points":34,"gross_equivalent":2,"net_equivalent":2,"actual_gross_total":88,"actual_net_total":88})
                );
            } else if player == seed::id(12) {
                assert_eq!(value["playing_handicap"], 40);
                assert_eq!(value["holes_scored"], 2);
                assert_eq!(value["holes"][1]["score"]["input"]["type"], "no_score");
                assert_eq!(value["holes"][1]["gross_points"], 0);
                assert_eq!(value["holes"][1]["net_points"], 0);
                assert!(value["holes"][1]["net_strokes"].is_null());
                assert!(value["holes"][2]["score"].is_null());
                assert!(value["holes"][2]["gross_points"].is_null());
                assert_eq!(
                    value["values"],
                    json!({"gross_points":2,"net_points":5,"gross_equivalent":2,"net_equivalent":-1,"actual_gross_total":null,"actual_net_total":null})
                );
            } else {
                assert_eq!(value["playing_handicap"], -10);
                assert_eq!(
                    value["values"],
                    json!({"gross_points":36,"net_points":26,"gross_equivalent":0,"net_equivalent":10,"actual_gross_total":72,"actual_net_total":82})
                );
            }
            assert!(value.get("confirmed_by").is_none());
            let entry = &value["holes"][0]["score"];
            assert_eq!(entry.get("revision").is_some(), scoring);
            assert_eq!(entry.get("submitted_by").is_some(), scoring);
            let suffix = if scoring { "/scoring" } else { "" };
            expected.push((
                format!("/api/rounds/{ROUND}/stableford/scorecards/{player}{suffix}"),
                value,
            ));
        }
    }
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
async fn roles_flights_resource_errors_and_adjacent_mutation_contracts_are_preserved(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    for role in ["admin", "scorer", "player", "viewer"] {
        sqlx::query("UPDATE tournament_memberships SET role=$1::text::tournament_role WHERE tournament_id=$2 AND user_id=$3").bind(role).bind(TRIP).bind(USER).execute(&pool).await.unwrap();
        for (index, path) in paths().iter().enumerate() {
            let r = get(&app, path, &token).await;
            assert_private(&r);
            let allowed = index == 0 || role == "admin" || role == "scorer";
            assert_eq!(
                r.status(),
                if allowed {
                    StatusCode::OK
                } else {
                    StatusCode::FORBIDDEN
                }
            );
            let v = body(r).await;
            if index == 0 {
                assert_eq!(
                    v["visible_hole_count"],
                    if role == "admin" { 18 } else { 9 }
                );
                if role != "admin" {
                    assert!(v["complete"].is_null());
                    assert!(v["confirmed"].is_null());
                }
            }
        }
    }
    seed::session(&pool, 41, Some(11), "player", "stableford-flight-player").await;
    for team in [PLAYER, seed::id(12), seed::id(13)] {
        for scoring in [false, true] {
            let suffix = if scoring { "/scoring" } else { "" };
            let path = format!("/api/rounds/{ROUND}/stableford/scorecards/{team}{suffix}");
            let r = get(&app, &path, "stableford-flight-player").await;
            assert_private(&r);
            assert_eq!(
                r.status(),
                if scoring && team == seed::id(13) {
                    StatusCode::FORBIDDEN
                } else {
                    StatusCode::OK
                }
            );
        }
    }
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
    }
    for target in [Uuid::new_v4().to_string(), "invalid".to_owned()] {
        for suffix in ["", "/scoring"] {
            let path = format!("/api/rounds/{target}/stableford/scorecards/{PLAYER}{suffix}");
            let r = get(&app, &path, &token).await;
            assert_private(&r);
            assert_eq!(
                r.status(),
                if target == "invalid" {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::NOT_FOUND
                }
            );
        }
    }
    for (method, path) in [
        (
            "PUT",
            format!("/api/rounds/{ROUND}/stableford/inputs/conditional"),
        ),
        (
            "POST",
            format!("/api/rounds/{ROUND}/stableford/scorecards/{PLAYER}/confirm"),
        ),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn hidden_input_and_confirmation_changes_do_not_leak_through_member_cards(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, _) = session(&pool).await;
    seed::fill(&pool, admin).await;
    seed::session(&pool, 41, None, "viewer", "stableford-hidden-viewer").await;
    let baseline = body(get(&app, &paths()[0], "stableford-hidden-viewer").await).await;
    assert_eq!(baseline["visible_hole_count"], 9);
    assert_eq!(baseline["values"]["gross_points"], 18);
    assert!(baseline["values"]["actual_gross_total"].is_null());
    assert!(baseline["complete"].is_null());
    assert!(baseline["confirmed"].is_null());
    for input in [Input::Numeric { gross_strokes: 20 }, Input::NoScore {}] {
        let card = stableford::get(&pool, admin, ROUND, PLAYER, true)
            .await
            .unwrap();
        let entry = card.holes[17].score.as_ref().unwrap();
        stableford::save_conditional(
            &pool,
            seed::operation(
                admin,
                11,
                18,
                input,
                ExpectedScore::Present {
                    score_id: entry.id,
                    revision: entry.revision,
                },
            ),
        )
        .await
        .unwrap();
        stableford::confirm(&pool, admin, ROUND, PLAYER)
            .await
            .unwrap();
        let before = score_state(&pool).await;
        let mut events = state.live_events.subscribe();
        let r = get(&app, &paths()[0], "stableford-hidden-viewer").await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_private(&r);
        assert_eq!(body(r).await, baseline);
        assert_eq!(score_state(&pool).await, before);
        assert!(events.try_recv().is_err());
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn format_owner_and_round_state_precedence_and_nine_hole_rejection_stay_intact(pool: PgPool) {
    let admin = seed::draft(&pool, "individual_stroke_play").await;
    let app = golf_api::api::router(golf_api::AppState::new(pool.clone()));
    for path in paths() {
        let r = get(&app, &path, seed::TOKEN).await;
        assert_eq!(r.status(), StatusCode::CONFLICT);
        assert_private(&r);
        assert_eq!(
            body(r).await["error"]["code"],
            "score_owner_format_mismatch"
        );
    }
    sqlx::query("UPDATE rounds SET scoring_format='individual_stableford' WHERE id=$1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE rounds SET number_of_holes=9 WHERE id=$1")
            .bind(ROUND)
            .execute(&pool)
            .await
            .is_err()
    );
    // A frozen player snapshot is resolved before editability, even in draft.
    for path in paths() {
        let r = get(&app, &path, seed::TOKEN).await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], "not_found");
    }
    golf_api::repositories::round_lifecycle::open_authorized(&pool, admin, ROUND)
        .await
        .unwrap();
    for suffix in ["", "/scoring"] {
        let r = get(
            &app,
            &format!(
                "/api/rounds/{ROUND}/stableford/scorecards/{}{suffix}",
                Uuid::new_v4()
            ),
            seed::TOKEN,
        )
        .await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_private(&r);
    }
    seed::fill(&pool, admin).await;
    for team in [PLAYER, seed::id(12), seed::id(13), seed::id(14)] {
        stableford::confirm(&pool, admin, ROUND, team)
            .await
            .unwrap();
    }
    golf_api::repositories::round_completion::complete(&pool, ROUND)
        .await
        .unwrap();
    assert_eq!(
        get(&app, &paths()[1], seed::TOKEN).await.status(),
        StatusCode::OK
    );
    golf_api::repositories::round_completion::lock(&pool, ROUND)
        .await
        .unwrap();
    assert_eq!(
        get(&app, &paths()[0], seed::TOKEN).await.status(),
        StatusCode::OK
    );
    let r = get(&app, &paths()[1], seed::TOKEN).await;
    assert_eq!(r.status(), StatusCode::CONFLICT);
    assert_eq!(body(r).await["error"]["code"], "round_not_editable");
}

#[sqlx::test(migrations = "../migrations")]
async fn late_confirmation_loading_keeps_inputs_and_confirmation_in_one_snapshot(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    seed::fill(&pool, admin).await;
    for path in paths() {
        stableford::confirm(&pool, admin, ROUND, PLAYER)
            .await
            .unwrap();
        let baseline = body(get(&app, &path, &token).await).await;
        assert_eq!(baseline["confirmed"], true);
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query("LOCK TABLE scorecard_confirmations IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *gate)
            .await
            .unwrap();
        let router = app.clone();
        let p = path.clone();
        let t = token.clone();
        let read = tokio::spawn(async move { get(&router, &p, &t).await });
        wait_blocked(&pool, pid).await;
        sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true),set_config('app.stableford_session_id',$2::text,true),set_config('app.stableford_actor_id',$3::text,true)").bind(ROUND).bind(admin).bind(USER).execute(&mut *gate).await.unwrap();
        sqlx::query("UPDATE stableford_inputs SET gross_strokes=gross_strokes+1,submitted_by=$3 WHERE round_id=$1 AND player_id=$2").bind(ROUND).bind(seed::id(11)).bind(USER).execute(&mut *gate).await.unwrap();
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
        assert_eq!(fresh["confirmed"], false);
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
