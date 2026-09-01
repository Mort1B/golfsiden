#![cfg(feature = "database-tests")]

use std::{sync::Arc, time::Duration as StdDuration};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use golf_api::{AppState, api, auth::hash_session_token, repositories::auth};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tokio::time::timeout;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("92000000-0000-0000-0000-000000000001");
const OTHER_TOURNAMENT: Uuid = uuid!("92000000-0000-0000-0000-000000000002");
const ADMIN: Uuid = uuid!("92000000-0000-0000-0000-000000000101");
const SCORER: Uuid = uuid!("92000000-0000-0000-0000-000000000102");
const PLAYER: Uuid = uuid!("92000000-0000-0000-0000-000000000103");
const VIEWER: Uuid = uuid!("92000000-0000-0000-0000-000000000104");
const OUTSIDER: Uuid = uuid!("92000000-0000-0000-0000-000000000105");

const MEMBERS: [(Uuid, &str, &str); 4] = [
    (ADMIN, "admin", "live-admin-token"),
    (SCORER, "scorer", "live-scorer-token"),
    (PLAYER, "player", "live-player-token"),
    (VIEWER, "viewer", "live-viewer-token"),
];
const OUTSIDER_TOKEN: &str = "live-global-admin-outsider-token";

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name, role) VALUES
        ('92000000-0000-0000-0000-000000000101', 'live_admin', 'Admin', 'viewer'),
        ('92000000-0000-0000-0000-000000000102', 'live_scorer', 'Scorer', 'admin'),
        ('92000000-0000-0000-0000-000000000103', 'live_player', 'Player', 'admin'),
        ('92000000-0000-0000-0000-000000000104', 'live_viewer', 'Viewer', 'admin'),
        ('92000000-0000-0000-0000-000000000105', 'live_outsider', 'Outsider', 'admin');
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds) VALUES
        ('92000000-0000-0000-0000-000000000001', 'Live Cup', '2026-08-01', '2026-08-02', 1),
        ('92000000-0000-0000-0000-000000000002', 'Other Live Cup', '2026-08-01', '2026-08-02', 1);
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('92000000-0000-0000-0000-000000000001', '92000000-0000-0000-0000-000000000101', 'admin'),
        ('92000000-0000-0000-0000-000000000001', '92000000-0000-0000-0000-000000000102', 'scorer'),
        ('92000000-0000-0000-0000-000000000001', '92000000-0000-0000-0000-000000000103', 'player'),
        ('92000000-0000-0000-0000-000000000001', '92000000-0000-0000-0000-000000000104', 'viewer'),
        ('92000000-0000-0000-0000-000000000002', '92000000-0000-0000-0000-000000000104', 'viewer');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    for (user_id, _, token) in MEMBERS
        .into_iter()
        .chain([(OUTSIDER, "outsider", OUTSIDER_TOKEN)])
    {
        auth::create_session(
            pool,
            user_id,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    }
}

fn live_request(tournament_id: Uuid, token: Option<&str>) -> Request<Body> {
    let mut request = Request::get(format!("/api/tournaments/{tournament_id}/live"));
    if let Some(token) = token {
        request = request.header(header::COOKIE, format!("golf_session={token}"));
    }
    request.body(Body::empty()).unwrap()
}

async fn error_code(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["error"]["code"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[sqlx::test(migrations = "../migrations")]
async fn live_handshake_requires_exact_membership_for_every_role(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool));

    for (_, role, token) in MEMBERS {
        let response = app
            .clone()
            .oneshot(live_request(TOURNAMENT, Some(token)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{role}");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/event-stream"
        );
    }

    let unauthenticated = app
        .clone()
        .oneshot(live_request(TOURNAMENT, None))
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(error_code(unauthenticated).await, "unauthenticated");

    let forbidden = app
        .clone()
        .oneshot(live_request(TOURNAMENT, Some(OUTSIDER_TOKEN)))
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    assert_eq!(error_code(forbidden).await, "forbidden");

    let missing = app
        .clone()
        .oneshot(live_request(Uuid::new_v4(), Some("live-admin-token")))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(error_code(missing).await, "not_found");

    let retired = app
        .oneshot(
            Request::get("/api/live")
                .header(header::COOKIE, "golf_session=live-admin-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(retired.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../migrations")]
async fn live_events_are_tournament_isolated_and_carry_only_constant_data(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool);
    let app = api::router(Arc::clone(&state));
    let response = app
        .oneshot(live_request(TOURNAMENT, Some("live-viewer-token")))
        .await
        .unwrap();
    let mut body = response.into_body();
    let other_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    state.notify("score", OTHER_TOURNAMENT, other_id);
    state.notify("score", TOURNAMENT, target_id);

    let frame = timeout(StdDuration::from_secs(2), body.frame())
        .await
        .expect("target event should arrive")
        .expect("stream should remain open")
        .expect("event frame should be valid");
    let bytes = frame.into_data().expect("event must be a data frame");
    let event = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(event, "event: score\ndata: invalidate\n\n");
    assert!(!event.contains(&TOURNAMENT.to_string()));
    assert!(!event.contains(&OTHER_TOURNAMENT.to_string()));
    assert!(!event.contains(&target_id.to_string()));
    assert!(!event.contains(&other_id.to_string()));
}

#[sqlx::test(migrations = "../migrations")]
async fn lagged_live_stream_closes_instead_of_remaining_stale(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool);
    let app = api::router(Arc::clone(&state));
    let response = app
        .oneshot(live_request(TOURNAMENT, Some("live-viewer-token")))
        .await
        .unwrap();
    let mut body = response.into_body();

    for _ in 0..129 {
        state.notify("score", TOURNAMENT, Uuid::new_v4());
    }

    let stream_end = timeout(StdDuration::from_secs(2), body.frame())
        .await
        .expect("lagged stream should close");
    assert!(stream_end.is_none());
}

#[sqlx::test(migrations = "../migrations")]
async fn revoked_session_and_removed_membership_close_before_later_events(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));

    let revoked_response = app
        .clone()
        .oneshot(live_request(TOURNAMENT, Some("live-admin-token")))
        .await
        .unwrap();
    let mut revoked_body = revoked_response.into_body();
    let admin_session = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM user_sessions WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(ADMIN)
    .fetch_one(&pool)
    .await
    .unwrap();
    auth::revoke_session(&pool, admin_session).await.unwrap();
    state.notify("round", TOURNAMENT, Uuid::new_v4());
    let revoked_end = timeout(StdDuration::from_secs(2), revoked_body.frame())
        .await
        .expect("revoked stream should close");
    assert!(revoked_end.is_none());

    let removed_response = app
        .oneshot(live_request(TOURNAMENT, Some("live-scorer-token")))
        .await
        .unwrap();
    let mut removed_body = removed_response.into_body();
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id = $1 AND user_id = $2")
        .bind(TOURNAMENT)
        .bind(SCORER)
        .execute(&pool)
        .await
        .unwrap();
    state.notify("round", TOURNAMENT, Uuid::new_v4());
    let removed_end = timeout(StdDuration::from_secs(2), removed_body.frame())
        .await
        .expect("removed-member stream should close");
    assert!(removed_end.is_none());
}
