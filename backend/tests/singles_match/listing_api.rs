use super::*;
use axum::{
    body::Body,
    http::{Request as HttpRequest, StatusCode},
};
use golf_api::{AppState, api};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

#[sqlx::test(migrations = "../migrations")]
async fn listing_http_validates_filter_identity_auth_and_unchanged_unfiltered_shape(pool: PgPool) {
    let (_, first, _) = ready(&pool).await;
    fixture::session(&pool, 205, None, "viewer", "listing-api-viewer").await;
    let app = api::router(AppState::new(pool.clone()));
    let base = format!("/api/rounds/{}/match-play/matches", id(5));
    for (suffix, token, status, count) in [
        (String::new(), TOKEN, StatusCode::OK, 2),
        (format!("?player_id={}", id(11)), TOKEN, StatusCode::OK, 1),
        (
            format!("?player_id={}", id(12)),
            "listing-api-viewer",
            StatusCode::OK,
            1,
        ),
        (format!("?player_id={}", id(999)), TOKEN, StatusCode::OK, 0),
        ("?player_id=wrong".into(), TOKEN, StatusCode::BAD_REQUEST, 0),
        ("?player_id=".into(), TOKEN, StatusCode::BAD_REQUEST, 0),
        (
            format!("?player_id={}&player_id={}", id(11), id(12)),
            TOKEN,
            StatusCode::BAD_REQUEST,
            0,
        ),
        (
            format!("?player_id={}", id(11)),
            "invalid-session",
            StatusCode::UNAUTHORIZED,
            0,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri(format!("{base}{suffix}"))
                    .header("cookie", format!("golf_session={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let value: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        if status == StatusCode::OK {
            assert_eq!(value["matches"].as_array().unwrap().len(), count);
            assert_eq!(value["round_id"], id(5).to_string());
            if suffix.is_empty() {
                assert!(value.get("player_id").is_none());
            } else {
                assert_eq!(value["player_id"], suffix.trim_start_matches("?player_id="));
            }
            if count == 1 {
                assert_eq!(value["matches"][0]["match_id"], first.to_string());
            }
        } else {
            assert!(value["error"]["code"].is_string());
        }
    }
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
        .bind(id(205))
        .execute(&pool)
        .await
        .unwrap();
    let response = app
        .oneshot(
            HttpRequest::builder()
                .uri(format!("{base}?player_id={}", id(11)))
                .header("cookie", "golf_session=listing-api-viewer")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
}
