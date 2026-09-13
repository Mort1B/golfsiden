use super::support::*;
use axum::http::StatusCode;
use chrono::{Duration, Utc};
use golf_api::{
    AppState,
    auth::{AuthConfig, hash_session_token},
    domain::password_recovery::RecoveryToken,
    rate_limit::{RateLimitRoute, RateLimiter},
    repositories::{auth, password_recovery},
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrations = "../migrations")]
async fn issue_preview_redeem_preserves_identity_and_invalidates_old_sessions(pool: PgPool) {
    fixture(&pool).await;
    auth::create_session(
        &pool,
        USER,
        &hash_session_token("target-session"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    let app = app(pool.clone());
    let response = app
        .clone()
        .oneshot(request(
            &admin_path(),
            json!({"current_password":PASSWORD}),
            true,
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let issued = body(response).await;
    assert_eq!(issued.as_object().unwrap().len(), 3);
    let id = issued["id"].as_str().unwrap().parse().unwrap();
    let url = issued["reset_url"].as_str().unwrap();
    assert!(url.starts_with(&format!("https://golf.example/reset-password/{id}#token=")));
    let token = url.split_once("#token=").unwrap().1;
    assert_eq!(token.len(), 43);
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT token_hash FROM password_recovery_grants WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(stored, token.as_bytes());
    assert!(
        auth::find_active_session(&pool, &hash_session_token("target-session"))
            .await
            .unwrap()
            .is_some()
    );
    for _ in 0..2 {
        let r = app
            .clone()
            .oneshot(request(
                &public_path(id, "preview"),
                json!({"token":token}),
                false,
                false,
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(
            body(r).await,
            json!({"id":issued["id"],"expires_at":issued["expires_at"]})
        );
    }
    let r = app
        .clone()
        .oneshot(request(
            &public_path(id, "redeem"),
            json!({"token":token,"new_password":NEW_PASSWORD,"confirm_password":NEW_PASSWORD}),
            true,
            false,
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert!(!r.headers().contains_key("set-cookie"));
    assert!(
        auth::find_active_session(&pool, &hash_session_token("target-session"))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        auth::find_active_session(&pool, &hash_session_token(TOKEN))
            .await
            .unwrap()
            .is_some()
    );
    let user = auth::find_login_user(&pool, "anders")
        .await
        .unwrap()
        .unwrap();
    assert!(
        golf_api::auth::verify_password(NEW_PASSWORD.into(), user.password_hash.unwrap()).await
    );
    let r = app
        .oneshot(request(
            &public_path(id, "preview"),
            json!({"token":token}),
            false,
            false,
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::CONFLICT);
    assert_eq!(body(r).await["error"]["code"], "password_recovery_invalid");
    let events: Vec<String> = sqlx::query_scalar(
        "SELECT outcome FROM password_recovery_audits WHERE grant_id=$1 ORDER BY id",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(events, ["issued", "redeemed"]);
    let linked: Option<uuid::Uuid> = sqlx::query_scalar("SELECT player_id FROM users WHERE id=$1")
        .bind(USER)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(linked, Some(USER));
}
#[sqlx::test(migrations = "../migrations")]
async fn admin_requires_session_csrf_password_and_strict_json(pool: PgPool) {
    fixture(&pool).await;
    let app = app(pool.clone());
    for (auth, csrf, password, status) in [
        (false, false, PASSWORD, StatusCode::UNAUTHORIZED),
        (true, false, PASSWORD, StatusCode::FORBIDDEN),
        (true, true, "wrong long password", StatusCode::CONFLICT),
    ] {
        let r = app
            .clone()
            .oneshot(request(
                &admin_path(),
                json!({"current_password":password}),
                auth,
                csrf,
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), status);
        assert_eq!(r.headers()["cache-control"], "private, no-store");
    }
    let r = app
        .clone()
        .oneshot(request(
            &admin_path(),
            json!({"current_password":PASSWORD,"user_id":OTHER}),
            true,
            true,
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    let (grant, token) = issue(
        &pool,
        &golf_api::repositories::password_recovery::AdminRequest {
            session_id: auth::find_active_session(&pool, &hash_session_token(TOKEN))
                .await
                .unwrap()
                .unwrap()
                .session_id,
            user_id: ADMIN,
            tournament_id: TRIP,
            player_id: USER,
            verified_password: golf_api::repositories::profile::credential(&pool, ADMIN)
                .await
                .unwrap()
                .unwrap(),
        },
    )
    .await;
    for (password, repeat) in [("short", "short"), (NEW_PASSWORD, "different password")] {
        assert_eq!(app.clone().oneshot(request(&public_path(grant.id,"redeem"),json!({"token":token.expose(),"new_password":password,"confirm_password":repeat}),false,false)).await.unwrap().status(),StatusCode::BAD_REQUEST);
    }
    for token in [
        "invalid".to_owned(),
        RecoveryToken::generate().unwrap().expose().to_owned(),
    ] {
        let r = app
            .clone()
            .oneshot(request(
                &public_path(grant.id, "preview"),
                json!({"token":token}),
                false,
                false,
            ))
            .await
            .unwrap();
        assert_eq!(body(r).await["error"]["code"], "password_recovery_invalid");
    }
    for _ in 0..2 {
        assert_eq!(
            app.clone()
                .oneshot(request(
                    &(admin_path() + "/revoke"),
                    json!({"current_password":PASSWORD}),
                    true,
                    true
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
    }
    assert!(
        password_recovery::preview(&pool, grant.id, &token)
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn every_recovery_route_is_throttled_and_issue_needs_explicit_origin(pool: PgPool) {
    let request_data = fixture(&pool).await;
    let (grant, token) = issue(&pool, &request_data).await;
    let state = AppState::with_runtime_services(
        pool.clone(),
        AuthConfig::local(),
        golf_api::course_provider::CourseProviderClient::disabled(),
        RateLimiter::with_rules(
            [
                RateLimitRoute::RecoveryAdmin,
                RateLimitRoute::RecoveryPreview,
                RateLimitRoute::RecoveryRedeem,
            ]
            .map(|route| (route, std::time::Duration::from_secs(60), 1, 2)),
            100,
        ),
    );
    let app = golf_api::api::router(state);
    for (path, data, authenticated, first_status) in [
        (
            admin_path(),
            json!({"current_password":PASSWORD}),
            true,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            public_path(grant.id, "preview"),
            json!({"token":token.expose()}),
            false,
            StatusCode::OK,
        ),
        (
            public_path(grant.id, "redeem"),
            json!({"token":"invalid","new_password":NEW_PASSWORD,"confirm_password":NEW_PASSWORD}),
            false,
            StatusCode::CONFLICT,
        ),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(&path, data.clone(), authenticated, authenticated))
                .await
                .unwrap()
                .status(),
            first_status
        );
        let r = app
            .clone()
            .oneshot(request(&path, data, authenticated, authenticated))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(r.headers().contains_key("retry-after"));
        assert_eq!(r.headers()["cache-control"], "private, no-store");
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn malformed_identifiers_keep_generic_json_and_private_headers(pool: PgPool) {
    fixture(&pool).await;
    let app = app(pool);
    for path in [
        "/api/auth/password-recovery/not-a-uuid/preview".to_owned(),
        format!("/api/tournaments/not-a-uuid/players/{USER}/password-recovery"),
    ] {
        let r = app
            .clone()
            .oneshot(request(
                &path,
                json!({"token":"invalid","current_password":PASSWORD}),
                true,
                true,
            ))
            .await
            .unwrap();
        assert_eq!(r.headers()["cache-control"], "private, no-store");
        assert!(body(r).await["error"]["code"].is_string());
    }
}
