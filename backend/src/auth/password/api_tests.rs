use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Days, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use super::{PASSWORD_WORK_LIMIT, hash_password_bounded, tests::PASSWORD_TEST_LOCK};
use crate::{AppState, api};

async fn post(app: axum::Router, path: &str, body: Value) -> axum::response::Response {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        app.oneshot(
            Request::post(path)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        ),
    )
    .await
    .unwrap()
    .unwrap()
}

async fn assert_capacity_response(response: axum::response::Response) {
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!response.headers().contains_key(header::SET_COOKIE));
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap(),
        json!({"error":{"code":"internal_error","message":"internal server error"}})
    );
}

async fn counts(pool: &PgPool) -> Vec<(String, i64)> {
    sqlx::query_as(
        "SELECT relation, row_count FROM (
           SELECT 'handicap_history'::text AS relation, count(*) AS row_count FROM handicap_history
           UNION ALL SELECT 'players', count(*) FROM players
           UNION ALL SELECT 'rounds', count(*) FROM rounds
           UNION ALL SELECT 'tournament_handicap_history', count(*) FROM tournament_handicap_history
           UNION ALL SELECT 'tournament_invitations', count(*) FROM tournament_invitations
           UNION ALL SELECT 'tournament_memberships', count(*) FROM tournament_memberships
           UNION ALL SELECT 'tournament_players', count(*) FROM tournament_players
           UNION ALL SELECT 'tournaments', count(*) FROM tournaments
           UNION ALL SELECT 'user_sessions', count(*) FROM user_sessions
           UNION ALL SELECT 'users', count(*) FROM users
         ) counts ORDER BY relation",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn saturation_preserves_login_parity_no_writes_and_later_recovery(pool: PgPool) {
    let _guard = PASSWORD_TEST_LOCK.lock().await;
    let hash = hash_password_bounded("synthetic test password".to_owned())
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, username, display_name, role, password_hash) VALUES ($1, 'admission_user', 'Synthetic User', 'viewer', $2)")
        .bind(Uuid::new_v4()).bind(hash).execute(&pool).await.unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let before = counts(&pool).await;
    let held = PASSWORD_WORK_LIMIT
        .admitted
        .clone()
        .acquire_many_owned(8)
        .await
        .unwrap();

    // Known and unknown users must reach the same shared admission boundary.
    for username in ["admission_user", "missing_user"] {
        assert_capacity_response(
            post(
                app.clone(),
                "/api/auth/login",
                json!({"username":username,"password":"synthetic test password"}),
            )
            .await,
        )
        .await;
    }
    let date = Utc::now()
        .date_naive()
        .checked_add_days(Days::new(2))
        .unwrap();
    let onboarding = json!({
        "creator": {
            "account":{"username":"admission_creator","password":"synthetic test password"},
            "player":{"display_name":"Synthetic Creator","handicap_index":12.3}
        },
        "tournament":{"name":"Synthetic Trip","description":"","start_date":date,"end_date":date,"counted_rounds":1,"mandatory_round_number":null},
        "rounds":[{"round_number":1,"name":"Opening","round_date":date,"scoring_format":"individual_stroke_play"}]
    });
    assert_capacity_response(
        post(
            app.clone(),
            "/api/onboarding/tournaments",
            onboarding.clone(),
        )
        .await,
    )
    .await;
    assert_eq!(counts(&pool).await, before);
    drop(held);

    let response = post(
        app.clone(),
        "/api/auth/login",
        json!({"username":"admission_user","password":"synthetic test password"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key(header::SET_COOKIE));
    for username in ["admission_user", "missing_user"] {
        let response = post(
            app.clone(),
            "/api/auth/login",
            json!({"username":username,"password":"incorrect password"}),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(!response.headers().contains_key(header::SET_COOKIE));
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            json!({"error":{"code":"invalid_credentials","message":"invalid username or password"}})
        );
    }
    assert_eq!(
        post(app, "/api/onboarding/tournaments", onboarding)
            .await
            .status(),
        StatusCode::CREATED
    );
    let (users, sessions): (i64, i64) =
        sqlx::query_as("SELECT (SELECT count(*) FROM users), (SELECT count(*) FROM user_sessions)")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((users, sessions), (2, 2));
    assert_eq!(PASSWORD_WORK_LIMIT.active.available_permits(), 4);
    assert_eq!(PASSWORD_WORK_LIMIT.admitted.available_permits(), 8);
}
