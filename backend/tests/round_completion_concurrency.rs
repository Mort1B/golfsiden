#![cfg(feature = "database-tests")]

use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    domain::scorecards::ScoreOwner,
    repositories::{auth, round_completion, round_lifecycle, scorecards},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{sync::broadcast::error::TryRecvError, task::JoinHandle};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const USER_ID: Uuid = uuid!("70000000-0000-0000-0000-000000000001");
const ROUND_ID: Uuid = uuid!("70000000-0000-0000-0000-000000000002");
const PLAYER_ID: Uuid = uuid!("70000000-0000-0000-0000-000000000003");
const HOLE_ID: Uuid = uuid!("70000000-0000-0000-0000-000000000004");
const SESSION_TOKEN: &str = "round-completion-concurrency-token";

const FIXTURE: &str = r#"
INSERT INTO users (id, email, display_name, role)
VALUES ('70000000-0000-0000-0000-000000000001', 'race@example.test', 'Admin', 'admin');
INSERT INTO players (id, display_name, current_handicap_index)
VALUES ('70000000-0000-0000-0000-000000000003', 'Player', 8.0);
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, status)
VALUES ('70000000-0000-0000-0000-000000000010', 'Race', '2026-01-01', '2026-01-01', 1, 'active');
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
VALUES ('70000000-0000-0000-0000-000000000010', '70000000-0000-0000-0000-000000000003', 8.0);
INSERT INTO courses (id, name)
VALUES ('70000000-0000-0000-0000-000000000011', 'Course');
INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
VALUES ('70000000-0000-0000-0000-000000000012', '70000000-0000-0000-0000-000000000011', 'Tee', 113, 4.0);
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
VALUES ('70000000-0000-0000-0000-000000000004', '70000000-0000-0000-0000-000000000012', 1, 4, 1);
INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format)
VALUES ('70000000-0000-0000-0000-000000000002', '70000000-0000-0000-0000-000000000010', 1, 'Round', '2026-01-01', '70000000-0000-0000-0000-000000000011', 'Course', '70000000-0000-0000-0000-000000000012', 'Tee', 1, 'individual_stroke_play');
INSERT INTO teams (id, round_id, tournament_id, name)
VALUES ('70000000-0000-0000-0000-000000000013', '70000000-0000-0000-0000-000000000002', '70000000-0000-0000-0000-000000000010', 'Group');
INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id)
VALUES ('70000000-0000-0000-0000-000000000013', '70000000-0000-0000-0000-000000000002', '70000000-0000-0000-0000-000000000010', '70000000-0000-0000-0000-000000000003');
"#;

async fn seed_ready_completed(pool: &PgPool) {
    sqlx::raw_sql(FIXTURE).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ('70000000-0000-0000-0000-000000000010', $1, 'admin')",
    )
    .bind(USER_ID)
    .execute(pool)
    .await
    .unwrap();
    round_lifecycle::open(pool, ROUND_ID).await.unwrap();
    scorecards::save(
        pool,
        scorecards::SaveScore {
            round_id: ROUND_ID,
            hole_id: HOLE_ID,
            owner: ScoreOwner::Player { id: PLAYER_ID },
            gross_strokes: 4,
            submitted_by: USER_ID,
        },
    )
    .await
    .unwrap();
    scorecards::confirm(
        pool,
        ROUND_ID,
        ScoreOwner::Player { id: PLAYER_ID },
        USER_ID,
    )
    .await
    .unwrap();
    round_completion::complete(pool, ROUND_ID).await.unwrap();
    auth::create_session(
        pool,
        USER_ID,
        &hash_session_token(SESSION_TOKEN),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
}

fn lock_request() -> Request<Body> {
    Request::post(format!("/api/rounds/{ROUND_ID}/lock"))
        .header("cookie", format!("golf_session={SESSION_TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(SESSION_TOKEN))
        .body(Body::empty())
        .unwrap()
}

fn save_request(strokes: i16) -> Request<Body> {
    Request::put(format!("/api/rounds/{ROUND_ID}/scores"))
        .header("content-type", "application/json")
        .header("cookie", format!("golf_session={SESSION_TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(SESSION_TOKEN))
        .body(Body::from(
            json!({
                "hole_id": HOLE_ID,
                "owner": {"type": "player", "id": PLAYER_ID},
                "gross_strokes": strokes
            })
            .to_string(),
        ))
        .unwrap()
}

fn confirm_request() -> Request<Body> {
    Request::post(format!(
        "/api/rounds/{ROUND_ID}/scorecards/player/{PLAYER_ID}/confirm"
    ))
    .header("content-type", "application/json")
    .header("cookie", format!("golf_session={SESSION_TOKEN}"))
    .header("x-csrf-token", derive_csrf_token(SESSION_TOKEN))
    .body(Body::from("{}"))
    .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn assert_waiting(
    handle: &mut JoinHandle<Result<axum::response::Response, std::convert::Infallible>>,
) {
    assert!(
        tokio::time::timeout(Duration::from_millis(100), handle)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_locks_commit_once_and_emit_one_sse(pool: PgPool) {
    seed_ready_completed(&pool).await;
    let state = AppState::new(pool);
    let app = api::router(Arc::clone(&state));
    let mut events = state.live_events.subscribe();
    let first_app = app.clone();
    let first = tokio::spawn(async move { first_app.oneshot(lock_request()).await });
    let second = tokio::spawn(async move { app.oneshot(lock_request()).await });
    let first = first.await.unwrap().unwrap();
    let second = second.await.unwrap().unwrap();
    let (success, conflict) = if first.status() == StatusCode::OK {
        (first, second)
    } else {
        (second, first)
    };
    assert_eq!(success.status(), StatusCode::OK);
    assert_eq!(response_json(success).await["status"], "locked");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(conflict).await,
        json!({"error": {"code": "round_not_completed", "message": "round must be completed to lock"}})
    );
    assert_eq!(events.try_recv().unwrap().id, ROUND_ID);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}

#[sqlx::test(migrations = "../migrations")]
async fn locking_round_forces_waiting_score_operations_to_recheck_locked_state(pool: PgPool) {
    seed_ready_completed(&pool).await;
    let mut lifecycle = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(ROUND_ID)
        .fetch_one(&mut *lifecycle)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.round_lock_id', $1::text, true)")
        .bind(ROUND_ID)
        .execute(&mut *lifecycle)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET status = 'locked' WHERE id = $1")
        .bind(ROUND_ID)
        .execute(&mut *lifecycle)
        .await
        .unwrap();

    let state = AppState::new(pool);
    let app = api::router(Arc::clone(&state));
    let mut events = state.live_events.subscribe();
    let same_app = app.clone();
    let correction_app = app.clone();
    let mut same = tokio::spawn(async move { same_app.oneshot(save_request(4)).await });
    let mut correction = tokio::spawn(async move { correction_app.oneshot(save_request(5)).await });
    let mut confirmation = tokio::spawn(async move { app.oneshot(confirm_request()).await });
    assert_waiting(&mut same).await;
    assert_waiting(&mut correction).await;
    assert_waiting(&mut confirmation).await;
    lifecycle.commit().await.unwrap();

    for response in [
        same.await.unwrap().unwrap(),
        correction.await.unwrap().unwrap(),
        confirmation.await.unwrap().unwrap(),
    ] {
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            response_json(response).await,
            json!({"error": {"code": "round_not_editable", "message": "scores require an open or completed round"}})
        );
    }
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}
