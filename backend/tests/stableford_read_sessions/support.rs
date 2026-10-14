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
use uuid::Uuid;
pub const USER: Uuid = Uuid::from_u128(1);
pub const TRIP: Uuid = Uuid::from_u128(2);
pub const ROUND: Uuid = Uuid::from_u128(5);
pub const PLAYER: Uuid = Uuid::from_u128(11);
pub async fn fixture(pool: &PgPool) -> Arc<AppState> {
    crate::stableford_fixture::ready(pool).await;
    AppState::new(pool.clone())
}
pub fn app(state: &Arc<AppState>) -> Router {
    api::router(Arc::clone(state))
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
    let path = format!("/api/rounds/{ROUND}/stableford/scorecards/{PLAYER}");
    vec![path.clone(), format!("{path}/scoring")]
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

pub async fn wait_blocked(pool: &PgPool, blocker: i32) -> i32 {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let pid: Option<i32> = sqlx::query_scalar("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) ORDER BY pid LIMIT 1").bind(blocker).fetch_optional(pool).await.unwrap();
            if let Some(pid) = pid { return pid; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("read must reach controlled database wait")
}

pub async fn lock_membership(tx: &mut Transaction<'_, Postgres>) {
    sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE").bind(TRIP).bind(USER).execute(&mut **tx).await.unwrap();
}

pub fn assert_private(response: &axum::response::Response) {
    assert!(
        response.headers()[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
}

pub async fn expire_at_wait(pool: &PgPool, session: Uuid, blocker_pid: i32) {
    wait_blocked(pool, blocker_pid).await;
    let active: bool =
        sqlx::query_scalar("SELECT expires_at>clock_timestamp() FROM user_sessions WHERE id=$1")
            .bind(session)
            .fetch_one(pool)
            .await
            .unwrap();
    assert!(active, "must observe real wait while session is active");
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

pub async fn score_state(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object(
      'inputs',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM stableford_inputs s),
      'audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM stableford_input_audits a),
      'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY user_id,request_id) FROM stableford_mutation_receipts r),
      'confirmations',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM scorecard_confirmations c),
      'handicaps',(SELECT jsonb_agg(to_jsonb(s) ORDER BY round_id,player_id) FROM round_handicap_snapshots s),
      'scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scores s),
      'score_audits',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM score_audits s)
    )").fetch_one(pool).await.unwrap()
}
