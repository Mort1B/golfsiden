#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{
    AppState, api,
    auth::{hash_password, hash_session_token},
    repositories::auth as auth_repository,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const USER_ID: Uuid = uuid!("71000000-0000-0000-0000-000000000001");
const PLAYER_ID: Uuid = uuid!("71000000-0000-0000-0000-000000000002");

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_user(pool: &PgPool) {
    let password_hash = hash_password(b"test-password").unwrap();
    sqlx::query(
        "INSERT INTO players (id, display_name, current_handicap_index, email)
         VALUES ($1, 'Session Player', 12.0, 'player@example.test')",
    )
    .bind(PLAYER_ID)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, role, player_id)
         VALUES ($1, 'player@example.test', 'Session Player', $2, 'player', $3)",
    )
    .bind(USER_ID)
    .bind(password_hash)
    .bind(PLAYER_ID)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn login_session_csrf_and_logout_follow_the_cookie_contract(pool: PgPool) {
    seed_user(&pool).await;
    let app = api::router(AppState::new(pool.clone()));

    let invalid = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"email": "player@example.test", "password": "wrong"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(invalid.headers()[header::CACHE_CONTROL], "no-store");
    let invalid_body = response_json(invalid).await;
    let unknown = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"email": "missing@example.test", "password": "wrong"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response_json(unknown).await, invalid_body);

    let login = app
        .clone()
        .oneshot(
            Request::post("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"email": " PLAYER@EXAMPLE.TEST ", "password": "test-password"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    assert_eq!(login.headers()[header::CACHE_CONTROL], "no-store");
    let set_cookie = login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Path=/api"));
    let cookie = set_cookie.split(';').next().unwrap().to_owned();
    let token = cookie.strip_prefix("golf_session=").unwrap();
    let body = response_json(login).await;
    let csrf = body["csrf_token"].as_str().unwrap();
    assert_eq!(body["role"], "player");
    assert_eq!(body["player_id"], PLAYER_ID.to_string());

    let stored_hash =
        sqlx::query_scalar::<_, Vec<u8>>("SELECT token_hash FROM user_sessions WHERE user_id = $1")
            .bind(USER_ID)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored_hash, hash_session_token(token));
    assert_ne!(stored_hash, token.as_bytes());

    let current = app
        .clone()
        .oneshot(
            Request::get("/api/auth/session")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(current.status(), StatusCode::OK);

    let no_csrf = app
        .clone()
        .oneshot(
            Request::post("/api/auth/logout")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_csrf.status(), StatusCode::FORBIDDEN);

    let logout = app
        .clone()
        .oneshot(
            Request::post("/api/auth/logout")
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    assert!(
        logout.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );

    let revoked = app
        .oneshot(
            Request::get("/api/auth/session")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(revoked.headers()[header::CACHE_CONTROL], "no-store");
}

#[sqlx::test(migrations = "../migrations")]
async fn email_identity_is_case_insensitively_unique(pool: PgPool) {
    seed_user(&pool).await;
    let duplicate = sqlx::query(
        "INSERT INTO users (id, email, display_name, role)
         VALUES ($1, 'PLAYER@example.test', 'Duplicate', 'viewer')",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(duplicate.as_database_error().unwrap().is_unique_violation());
}

#[sqlx::test(migrations = "../migrations")]
async fn logout_revocation_waits_for_an_authorized_transaction(pool: PgPool) {
    seed_user(&pool).await;
    let raw_token = "concurrent-session-token";
    let principal = auth_repository::create_session(
        &pool,
        USER_ID,
        &hash_session_token(raw_token),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
    let mut score_transaction = pool.begin().await.unwrap();
    auth_repository::lock_active_session(&mut score_transaction, principal.session_id)
        .await
        .unwrap()
        .unwrap();

    let revoke_pool = pool.clone();
    let mut revoke = tokio::spawn(async move {
        auth_repository::revoke_session(&revoke_pool, principal.session_id).await
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut revoke)
            .await
            .is_err()
    );
    score_transaction.commit().await.unwrap();
    revoke.await.unwrap().unwrap();

    assert!(
        auth_repository::find_active_session(&pool, &hash_session_token(raw_token))
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn expired_sessions_are_rejected(pool: PgPool) {
    seed_user(&pool).await;
    let raw_token = "expired-session-token";
    let principal = auth_repository::create_session(
        &pool,
        USER_ID,
        &hash_session_token(raw_token),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE user_sessions
         SET created_at = now() - interval '2 hours', expires_at = now() - interval '1 hour'
         WHERE id = $1",
    )
    .bind(principal.session_id)
    .execute(&pool)
    .await
    .unwrap();

    let response = api::router(AppState::new(pool))
        .oneshot(
            Request::get("/api/auth/session")
                .header(header::COOKIE, format!("golf_session={raw_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
