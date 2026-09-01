#![cfg(feature = "database-tests")]

use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use axum::{
    Router,
    body::{Body, Bytes},
    http::{Request, StatusCode, header, request::Builder},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::auth,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::{broadcast::error::TryRecvError, oneshot};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT_ID: Uuid = uuid!("91000000-0000-0000-0000-000000000001");
const MISSING_TOURNAMENT_ID: Uuid = uuid!("91000000-0000-0000-0000-000000000099");
const ADMIN: Uuid = uuid!("91000000-0000-0000-0000-000000000011");
const SCORER: Uuid = uuid!("91000000-0000-0000-0000-000000000012");
const PLAYER: Uuid = uuid!("91000000-0000-0000-0000-000000000013");
const VIEWER: Uuid = uuid!("91000000-0000-0000-0000-000000000014");
const OUTSIDER: Uuid = uuid!("91000000-0000-0000-0000-000000000015");
const CROSS_TOURNAMENT_ADMIN: Uuid = uuid!("91000000-0000-0000-0000-000000000016");
const GLOBAL_ADMIN: Uuid = uuid!("91000000-0000-0000-0000-000000000017");
const EXPIRED_ADMIN: Uuid = uuid!("91000000-0000-0000-0000-000000000018");

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name, role) VALUES
        ('91000000-0000-0000-0000-000000000011', 'round_admin', 'Round admin', 'viewer'),
        ('91000000-0000-0000-0000-000000000012', 'round_scorer', 'Round scorer', 'admin'),
        ('91000000-0000-0000-0000-000000000013', 'round_player', 'Round player', 'admin'),
        ('91000000-0000-0000-0000-000000000014', 'round_viewer', 'Round viewer', 'admin'),
        ('91000000-0000-0000-0000-000000000015', 'round_outsider', 'Round outsider', 'viewer'),
        ('91000000-0000-0000-0000-000000000016', 'round_cross_admin', 'Cross admin', 'admin'),
        ('91000000-0000-0000-0000-000000000017', 'round_global_admin', 'Global admin', 'admin'),
        ('91000000-0000-0000-0000-000000000018', 'round_expired_admin', 'Expired admin', 'viewer');

        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, counted_rounds)
        VALUES
        ('91000000-0000-0000-0000-000000000001', 'Authorization target', '2026-09-01', '2026-09-04', 4, 4),
        ('91000000-0000-0000-0000-000000000002', 'Other target', '2026-10-01', '2026-10-02', 2, 2);

        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000011', 'admin'),
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000012', 'scorer'),
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000013', 'player'),
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000014', 'viewer'),
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000018', 'admin'),
        ('91000000-0000-0000-0000-000000000002', '91000000-0000-0000-0000-000000000016', 'admin');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    for (user_id, token) in [
        (ADMIN, "round-admin-token"),
        (SCORER, "round-scorer-token"),
        (PLAYER, "round-player-token"),
        (VIEWER, "round-viewer-token"),
        (OUTSIDER, "round-outsider-token"),
        (CROSS_TOURNAMENT_ADMIN, "round-cross-token"),
        (GLOBAL_ADMIN, "round-global-token"),
    ] {
        auth::create_session(
            pool,
            user_id,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    }

    sqlx::query(
        "INSERT INTO user_sessions (id, user_id, token_hash, created_at, expires_at)
         VALUES ($1, $2, $3, now() - interval '2 hours', now() - interval '1 hour')",
    )
    .bind(Uuid::new_v4())
    .bind(EXPIRED_ADMIN)
    .bind(hash_session_token("round-expired-token"))
    .execute(pool)
    .await
    .unwrap();
}

fn with_session(builder: Builder, token: &str) -> Builder {
    builder.header(header::COOKIE, format!("golf_session={token}"))
}

fn authorized(builder: Builder, token: &str) -> Builder {
    with_session(builder, token).header("x-csrf-token", derive_csrf_token(token))
}

fn request(builder: Builder, body: impl Into<Body>) -> Request<Body> {
    builder.body(body.into()).unwrap()
}

fn json_request(builder: Builder, token: &str, body: Value) -> Request<Body> {
    request(
        authorized(builder, token).header(header::CONTENT_TYPE, "application/json"),
        body.to_string(),
    )
}

fn tracked_body(polled: Arc<AtomicBool>) -> Body {
    Body::from_stream(futures_util::stream::once(async move {
        polled.store(true, Ordering::SeqCst);
        Ok::<_, Infallible>(Bytes::from_static(b"{"))
    }))
}

fn gated_json_request(
    builder: Builder,
    token: &str,
    value: Value,
) -> (Request<Body>, oneshot::Receiver<()>, oneshot::Sender<()>) {
    let (polled_tx, polled_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    let stream = futures_util::stream::once(async move {
        let _ = polled_tx.send(());
        let _ = resume_rx.await;
        Ok::<_, Infallible>(Bytes::from(value.to_string()))
    });
    let request = authorized(builder, token)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from_stream(stream))
        .unwrap();
    (request, polled_rx, resume_tx)
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn round_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM rounds WHERE tournament_id = $1")
        .bind(TOURNAMENT_ID)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn assert_rejected_without_effect(
    app: &Router,
    pool: &PgPool,
    events: &mut tokio::sync::broadcast::Receiver<golf_api::LiveEvent>,
    request: Request<Body>,
    expected_status: StatusCode,
    expected_code: &str,
) -> Value {
    let before = round_count(pool).await;
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), expected_status);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], expected_code);
    assert_eq!(round_count(pool).await, before);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    body
}

fn valid_round() -> Value {
    json!({
        "round_number": 1,
        "name": "Opening round",
        "round_date": "2026-09-01",
        "course_name": "Hacienda del Alamo",
        "tee_name": "Yellow",
        "scoring_format": "individual_stroke_play"
    })
}

#[sqlx::test(migrations = "../migrations")]
async fn authentication_and_csrf_precede_every_body_error(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let path = format!("/api/tournaments/{TOURNAMENT_ID}/rounds");

    let cases = [
        (
            request(Request::post(&path), "{"),
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
        ),
        (
            request(with_session(Request::post(&path), "unknown-token"), "{"),
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
        ),
        (
            request(
                authorized(Request::post(&path), "round-expired-token"),
                "{\"unknown\":true}",
            ),
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
        ),
        (
            request(with_session(Request::post(&path), "round-admin-token"), "{"),
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
        (
            request(
                with_session(Request::post(&path), "round-admin-token")
                    .header("x-csrf-token", "invalid"),
                "{\"round_number\":99}",
            ),
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
    ];

    for (request, status, code) in cases {
        assert_rejected_without_effect(&app, &pool, &mut events, request, status, code).await;
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_admin_preflight_precedes_transport_schema_and_semantic_validation(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let path = format!("/api/tournaments/{TOURNAMENT_ID}/rounds");

    let body_polled = Arc::new(AtomicBool::new(false));
    let forbidden_without_body_poll = authorized(Request::post(&path), "round-scorer-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(tracked_body(Arc::clone(&body_polled)))
        .unwrap();
    assert_rejected_without_effect(
        &app,
        &pool,
        &mut events,
        forbidden_without_body_poll,
        StatusCode::FORBIDDEN,
        "forbidden",
    )
    .await;
    assert!(!body_polled.load(Ordering::SeqCst));

    let forbidden = [
        request(
            authorized(Request::post(&path), "round-scorer-token"),
            valid_round().to_string(),
        ),
        request(
            authorized(Request::post(&path), "round-player-token")
                .header(header::CONTENT_TYPE, "application/json"),
            "{",
        ),
        json_request(
            Request::post(&path),
            "round-viewer-token",
            json!({"unknown": true}),
        ),
        json_request(
            Request::post(&path),
            "round-outsider-token",
            json!({"round_number": 5}),
        ),
        json_request(
            Request::post(&path),
            "round-cross-token",
            json!({"course_id": Uuid::new_v4()}),
        ),
        json_request(
            Request::post(&path),
            "round-global-token",
            json!({"name": ""}),
        ),
    ];
    for request in forbidden {
        assert_rejected_without_effect(
            &app,
            &pool,
            &mut events,
            request,
            StatusCode::FORBIDDEN,
            "forbidden",
        )
        .await;
    }

    let missing = request(
        authorized(
            Request::post(format!("/api/tournaments/{MISSING_TOURNAMENT_ID}/rounds")),
            "round-admin-token",
        ),
        "{",
    );
    assert_rejected_without_effect(
        &app,
        &pool,
        &mut events,
        missing,
        StatusCode::NOT_FOUND,
        "not_found",
    )
    .await;

    let admin_validation_cases = [
        (
            request(
                authorized(Request::post(&path), "round-admin-token"),
                valid_round().to_string(),
            ),
            "content-type must be application/json",
        ),
        (
            request(
                authorized(Request::post(&path), "round-admin-token")
                    .header(header::CONTENT_TYPE, "application/json"),
                "{",
            ),
            "request body must contain valid round fields",
        ),
        (
            json_request(
                Request::post(&path),
                "round-admin-token",
                json!({"unknown": true}),
            ),
            "request body must contain valid round fields",
        ),
        (
            json_request(
                Request::post(&path),
                "round-admin-token",
                json!({
                    "round_number": 5,
                    "name": "Too many",
                    "round_date": "2026-09-01",
                    "course_name": "Course",
                    "tee_name": "Tee",
                    "scoring_format": "individual_stroke_play"
                }),
            ),
            "round_number must be between 1 and 4",
        ),
        (
            json_request(
                Request::post(&path),
                "round-admin-token",
                json!({
                    "round_number": 1,
                    "name": "Mismatched course",
                    "round_date": "2026-09-01",
                    "course_id": Uuid::new_v4(),
                    "course_name": "Course",
                    "tee_name": "Tee",
                    "scoring_format": "individual_stroke_play"
                }),
            ),
            "course_id and tee_id must be provided together",
        ),
    ];
    for (request, message) in admin_validation_cases {
        let body = assert_rejected_without_effect(
            &app,
            &pool,
            &mut events,
            request,
            StatusCode::BAD_REQUEST,
            "validation_error",
        )
        .await;
        assert_eq!(body["error"]["message"], message);
    }

    let failed_stream = futures_util::stream::once(async {
        Err::<Bytes, _>(std::io::Error::other("deliberate request-body failure"))
    });
    let unreadable = authorized(Request::post(&path), "round-admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from_stream(failed_stream))
        .unwrap();
    let body = assert_rejected_without_effect(
        &app,
        &pool,
        &mut events,
        unreadable,
        StatusCode::BAD_REQUEST,
        "validation_error",
    )
    .await;
    assert_eq!(body["error"]["message"], "request body could not be read");

    let oversized = authorized(Request::post(&path), "round-admin-token")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(vec![b'x'; 32 * 1024 + 1]))
        .unwrap();
    let body = assert_rejected_without_effect(
        &app,
        &pool,
        &mut events,
        oversized,
        StatusCode::PAYLOAD_TOO_LARGE,
        "payload_too_large",
    )
    .await;
    assert_eq!(body["error"]["message"], "request body is too large");
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_admin_create_commits_once_then_publishes_one_matching_event(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let request = authorized(
        Request::post(format!("/api/tournaments/{TOURNAMENT_ID}/rounds")),
        "round-admin-token",
    )
    .header(header::CONTENT_TYPE, "application/vnd.golf.round+JsOn")
    .body(Body::from(valid_round().to_string()))
    .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    let round_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    assert_eq!(body["tournament_id"], TOURNAMENT_ID.to_string());
    assert_eq!(round_count(&pool).await, 1);

    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "round");
    assert_eq!(event.tournament_id, TOURNAMENT_ID);
    assert_eq!(event.id, round_id);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}

#[sqlx::test(migrations = "../migrations")]
async fn create_rechecks_session_and_membership_after_successful_preflight(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let path = format!("/api/tournaments/{TOURNAMENT_ID}/rounds");

    let (request, preflight_finished, resume) =
        gated_json_request(Request::post(&path), "round-admin-token", valid_round());
    let request_task = tokio::spawn({
        let app = app.clone();
        async move { app.oneshot(request).await.unwrap() }
    });
    preflight_finished.await.unwrap();
    sqlx::query("UPDATE user_sessions SET revoked_at = now() WHERE token_hash = $1")
        .bind(hash_session_token("round-admin-token"))
        .execute(&pool)
        .await
        .unwrap();
    resume.send(()).unwrap();
    let response = request_task.await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "unauthenticated"
    );
    assert_eq!(round_count(&pool).await, 0);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    auth::create_session(
        &pool,
        EXPIRED_ADMIN,
        &hash_session_token("round-removal-token"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    let (request, preflight_finished, resume) =
        gated_json_request(Request::post(&path), "round-removal-token", valid_round());
    let request_task = tokio::spawn({
        let app = app.clone();
        async move { app.oneshot(request).await.unwrap() }
    });
    preflight_finished.await.unwrap();
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id = $1 AND user_id = $2")
        .bind(TOURNAMENT_ID)
        .bind(EXPIRED_ADMIN)
        .execute(&pool)
        .await
        .unwrap();
    resume.send(()).unwrap();
    let response = request_task.await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response_json(response).await["error"]["code"], "forbidden");
    assert_eq!(round_count(&pool).await, 0);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}
