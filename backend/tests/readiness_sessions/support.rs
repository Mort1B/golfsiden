use std::{sync::Arc, time::Duration};

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
use tower::ServiceExt;
use uuid::{Uuid, uuid};

pub const FIRST_PLAYER: Uuid = uuid!("00000000-0000-0000-0000-000000001001");
pub const USER: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
pub const FINAL: Uuid = uuid!("00000000-0000-0000-0000-000000004005");
pub const ROUND: Uuid = uuid!("00000000-0000-0000-0000-000000004001");
pub const TRIP: Uuid = uuid!("00000000-0000-0000-0000-000000002001");

pub async fn fixture(pool: &PgPool) -> Arc<AppState> {
    sqlx::raw_sql(include_str!("../../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
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
    vec![
        format!("/api/rounds/{ROUND}/pairing-validation"),
        format!("/api/rounds/{ROUND}/completion-validation"),
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

pub async fn empty_round(pool: &PgPool) -> (Uuid, Uuid) {
    let trip = Uuid::new_v4();
    let round = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Empty trip','2026-10-01','2026-10-01',1)").bind(trip).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'viewer')",
    )
    .bind(trip)
    .bind(USER)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES($1,$2,1,'Empty round','2026-10-01','','','team_scramble')").bind(round).bind(trip).execute(pool).await.unwrap();
    (trip, round)
}

pub async fn open_final(pool: &PgPool, session: Uuid) {
    let updated = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(pool)
        .await
        .unwrap();
    golf_api::repositories::tournaments::start_authorized(pool, session, TRIP, updated)
        .await
        .unwrap();
    golf_api::repositories::round_lifecycle::open_authorized(pool, session, FINAL)
        .await
        .unwrap();
}

pub async fn lock_loading(tx: &mut Transaction<'_, Postgres>, path: &str, empty: bool) {
    let sql = if path.ends_with("/pairing-validation") && empty {
        "LOCK TABLE flight_memberships IN ACCESS EXCLUSIVE MODE"
    } else {
        // Pairing's final configured-tee facts; completion's visible-hole query
        // after owner facts (owner fact queries do not read holes).
        "LOCK TABLE holes IN ACCESS EXCLUSIVE MODE"
    };
    sqlx::query(sql).execute(&mut **tx).await.unwrap();
}
