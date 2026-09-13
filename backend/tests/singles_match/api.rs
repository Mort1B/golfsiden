use super::*;
use axum::{
    body::Body,
    http::{Request as HttpRequest, StatusCode},
};
use golf_api::{AppState, api, auth::derive_csrf_token, domain::match_play::commands::Event};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
#[sqlx::test(migrations = "../migrations")]
async fn typed_http_auth_errors_no_store_and_locked_scoring_policy(pool: PgPool) {
    let (session, m, other) = ready(&pool).await;
    let scorer_token = "match-api-scorer";
    fixture::session(&pool, 204, Some(11), "scorer", scorer_token).await;
    let app = api::router(AppState::new(pool.clone()));
    let path = format!("/api/rounds/{}/match-play/matches/{m}/commands", id(5));
    let invalid = json!({"request_id":Uuid::new_v4(),"expected_revision":"1","command":{"type":"note","player_id":id(11),"hole_number":1,"gross_strokes":4,"unexpected":true}});
    let response = app
        .clone()
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri(&path)
                .header("cookie", format!("golf_session={TOKEN}"))
                .header("x-csrf-token", derive_csrf_token(TOKEN))
                .header("content-type", "application/json")
                .body(Body::from(invalid.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let request = request(
        1,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: id(12),
                communicated: true,
                after_hole: 0,
            },
        },
    );
    let response = app
        .clone()
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri(&path)
                .header("cookie", format!("golf_session={TOKEN}"))
                .header("x-csrf-token", derive_csrf_token(TOKEN))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["applied_revision"], "2");
    submit(
        &pool,
        session,
        m,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    submit(
        &pool,
        session,
        other,
        1,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: id(14),
                communicated: true,
                after_hole: 0,
            },
        },
    )
    .await;
    submit(
        &pool,
        session,
        other,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    golf_api::repositories::round_completion::complete_authorized(&pool, session, id(5))
        .await
        .unwrap();
    golf_api::repositories::round_completion::lock_authorized(&pool, session, id(5))
        .await
        .unwrap();
    sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden=TRUE WHERE id=$1")
        .bind(id(2))
        .execute(&pool)
        .await
        .unwrap();
    let scoring = format!("/api/rounds/{}/match-play/matches/{m}/scoring", id(5));
    for (token, status) in [
        (scorer_token, StatusCode::FORBIDDEN),
        (TOKEN, StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri(&scoring)
                    .header("cookie", format!("golf_session={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
}
