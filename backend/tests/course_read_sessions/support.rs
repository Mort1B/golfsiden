use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{AppState, api, auth::hash_session_token, repositories::auth};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

pub const USER: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
pub const TRIP: Uuid = uuid!("00000000-0000-0000-0000-000000002001");

pub async fn fixture(pool: &PgPool) -> (Arc<AppState>, Router) {
    sqlx::raw_sql(include_str!("../../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let router = api::router(Arc::clone(&state));
    (state, router)
}
pub async fn session(pool: &PgPool) -> (Uuid, String) {
    let token = Uuid::new_v4().to_string();
    let principal = auth::create_session(
        pool,
        USER,
        &hash_session_token(&token),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
    (principal.session_id, token)
}
pub fn paths() -> Vec<String> {
    vec![
        format!("/api/tournaments/{TRIP}/course-catalog"),
        format!("/api/tournaments/{TRIP}/course-presets"),
        format!("/api/tournaments/{TRIP}/course-provider/courses/dcm3cn0g"),
    ]
}
pub async fn get(app: &Router, path: &str, token: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::get(path)
                .header(header::COOKIE, format!("golf_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}
pub async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
pub async fn blocker(tx: &mut Transaction<'_, Postgres>) -> i32 {
    sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}
pub async fn lock_membership(tx: &mut Transaction<'_, Postgres>) {
    sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE").bind(TRIP).bind(USER).execute(&mut **tx).await.unwrap();
}
pub async fn wait_blocked(pool: &PgPool, blocker: i32) -> i32 {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let pid: Option<i32> = sqlx::query_scalar("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) ORDER BY pid LIMIT 1").bind(blocker).fetch_optional(pool).await.unwrap();
            if let Some(pid) = pid { return pid; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("course read must reach its controlled database wait")
}
pub async fn short_session(pool: &PgPool, session: Uuid) {
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1",
    )
    .bind(session)
    .execute(pool)
    .await
    .unwrap();
}
pub async fn wait_expired(pool: &PgPool, session: Uuid) {
    let active: bool =
        sqlx::query_scalar("SELECT expires_at>clock_timestamp() FROM user_sessions WHERE id=$1")
            .bind(session)
            .fetch_one(pool)
            .await
            .unwrap();
    assert!(active, "must observe database wait before natural expiry");
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let expired: bool = sqlx::query_scalar(
                "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1",
            )
            .bind(session)
            .fetch_one(pool)
            .await
            .unwrap();
            if expired {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
