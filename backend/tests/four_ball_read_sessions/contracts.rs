use super::support::*;
use crate::four_ball_fixture as seed;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{
    domain::{
        four_ball_card::{Input, read_projection},
        scorecards::{ExpectedScore, ScoreOwner},
    },
    repositories::{four_ball, score_authorization},
};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::Uuid;

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_full_payloads_preserve_partner_inputs_ties_and_frozen_handicaps(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, token) = session(&pool).await;
    seed::fill(&pool, admin).await;
    seed::numeric(&pool, admin, 12, 1, 4).await;
    four_ball::save_conditional(
        &pool,
        seed::operation(admin, 12, 2, Input::NoScore {}, ExpectedScore::Absent {}),
    )
    .await
    .unwrap();
    four_ball::confirm(&pool, admin, ROUND, TEAM).await.unwrap();
    let mut expected = Vec::new();
    for team in [TEAM, seed::id(22)] {
        for scoring in [false, true] {
            let card = four_ball::get(&pool, admin, ROUND, team, scoring)
                .await
                .unwrap();
            let value = if scoring {
                serde_json::to_value(card)
            } else {
                serde_json::to_value(read_projection(card))
            }
            .unwrap();
            if team == TEAM {
                assert_eq!(value["confirmed"], true);
                assert_eq!(value["partners"][1]["playing_handicap"], 34);
                assert_eq!(
                    value["holes"][0]["gross"]["player_ids"],
                    json!([seed::id(11), seed::id(12)])
                );
                assert_eq!(
                    value["holes"][0]["net"]["player_ids"],
                    json!([seed::id(12)])
                );
                assert_eq!(
                    value["holes"][1]["players"][1]["score"]["input"]["type"],
                    "no_score"
                );
                assert!(value["holes"][2]["players"][1]["score"].is_null());
            } else {
                assert_eq!(value["partners"][0]["playing_handicap"], -8);
            }
            assert!(value.get("confirmed_by").is_none());
            let entry = &value["holes"][0]["players"][0]["score"];
            assert_eq!(entry.get("revision").is_some(), scoring);
            assert_eq!(entry.get("submitted_by").is_some(), scoring);
            let suffix = if scoring { "/scoring" } else { "" };
            expected.push((
                format!("/api/rounds/{ROUND}/four-ball/scorecards/{team}{suffix}"),
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
    seed::session(&pool, 41, Some(11), "player", "four-ball-flight-player").await;
    for team in [TEAM, seed::id(22)] {
        for scoring in [false, true] {
            let suffix = if scoring { "/scoring" } else { "" };
            let path = format!("/api/rounds/{ROUND}/four-ball/scorecards/{team}{suffix}");
            let r = get(&app, &path, "four-ball-flight-player").await;
            assert_private(&r);
            assert_eq!(
                r.status(),
                if scoring && team != TEAM {
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
            let path = format!("/api/rounds/{target}/four-ball/scorecards/{TEAM}{suffix}");
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
            format!("/api/rounds/{ROUND}/four-ball/inputs/conditional"),
        ),
        (
            "POST",
            format!("/api/rounds/{ROUND}/four-ball/scorecards/{TEAM}/confirm"),
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
async fn one_authorized_partner_cannot_authorize_the_side_scoring_read(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let session = seed::session(&pool, 41, Some(11), "player", "single-partner-authority").await;
    // A schema-only read fixture hides the actor's flight link without changing
    // frozen pairings or bypassing production pairing guards on stored rows.
    sqlx::raw_sql("ALTER TABLE flight_memberships RENAME TO stored_flight_memberships;
      CREATE VIEW flight_memberships AS SELECT * FROM stored_flight_memberships WHERE player_id <> '00000000-0000-0000-0000-00000000000b'::uuid;").execute(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    let format = golf_api::domain::models::ScoringFormat::FourBallStrokePlay;
    assert!(
        score_authorization::authorize_mutation(
            &mut tx,
            session,
            ROUND,
            format,
            ScoreOwner::Player { id: seed::id(11) }
        )
        .await
        .is_ok()
    );
    assert!(matches!(
        score_authorization::authorize_mutation(
            &mut tx,
            session,
            ROUND,
            format,
            ScoreOwner::Player { id: seed::id(12) }
        )
        .await,
        Err(score_authorization::ScoreAuthorizationError::Forbidden)
    ));
    tx.rollback().await.unwrap();
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let r = get(&app, &paths()[0], "single-partner-authority").await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_private(&r);
    let r = get(&app, &paths()[1], "single-partner-authority").await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_private(&r);
    assert_eq!(body(r).await["error"]["code"], "forbidden");
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
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
    sqlx::query("UPDATE rounds SET scoring_format='four_ball_stroke_play' WHERE id=$1")
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
    // Frozen partners are required before editability is checked, even in draft.
    for path in paths() {
        let r = get(&app, &path, seed::TOKEN).await;
        assert_eq!(r.status(), StatusCode::CONFLICT);
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], "score_owner_not_eligible");
    }
    golf_api::repositories::round_lifecycle::open_authorized(&pool, admin, ROUND)
        .await
        .unwrap();
    for suffix in ["", "/scoring"] {
        let r = get(
            &app,
            &format!(
                "/api/rounds/{ROUND}/four-ball/scorecards/{}{suffix}",
                Uuid::new_v4()
            ),
            seed::TOKEN,
        )
        .await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_private(&r);
    }
    seed::fill(&pool, admin).await;
    for team in [TEAM, seed::id(22)] {
        four_ball::confirm(&pool, admin, ROUND, team).await.unwrap();
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
        four_ball::confirm(&pool, admin, ROUND, TEAM).await.unwrap();
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
        sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true),set_config('app.four_ball_session_id',$2::text,true),set_config('app.four_ball_actor_id',$3::text,true)").bind(ROUND).bind(admin).bind(USER).execute(&mut *gate).await.unwrap();
        sqlx::query("UPDATE four_ball_inputs SET gross_strokes=gross_strokes+1,submitted_by=$3 WHERE round_id=$1 AND player_id=$2").bind(ROUND).bind(seed::id(11)).bind(USER).execute(&mut *gate).await.unwrap();
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
