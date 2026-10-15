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
pub const INVITE: Uuid = Uuid::from_u128(51);
pub async fn fixture(pool: &PgPool) -> Arc<AppState> {
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,'invitation_read_admin','Admin','admin')").bind(USER).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Invitation metadata cup','2026-10-15','2026-10-15',1)").bind(TRIP).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(TRIP)
    .bind(USER)
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
pub fn path() -> String {
    format!("/api/tournaments/{TRIP}/invitations")
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
            .eq("no-store")
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

pub async fn read_state(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object(
      'invitations',(SELECT jsonb_agg(to_jsonb(i)-'token_hash' ORDER BY id) FROM tournament_invitations i),
      'users',(SELECT jsonb_agg(to_jsonb(u)-'password_hash' ORDER BY id) FROM users u),
      'redemptions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM invitation_redemptions r),
      'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY tournament_id,user_id) FROM tournament_memberships m),
      'entrants',(SELECT jsonb_agg(to_jsonb(p) ORDER BY tournament_id,player_id) FROM tournament_players p),
      'scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scores s)
    )").fetch_one(pool).await.unwrap()
}
pub async fn invitation(pool: &PgPool) -> String {
    let token = golf_api::auth::generate_invitation_token().unwrap();
    sqlx::query("INSERT INTO tournament_invitations(id,tournament_id,series_id,created_by_user_id,token_hash,expires_at,max_uses) VALUES($1,$2,$1,$3,$4,clock_timestamp()+interval '1 day',5)").bind(INVITE).bind(TRIP).bind(USER).bind(golf_api::auth::hash_invitation_token(&token).as_slice()).execute(pool).await.unwrap();
    token
}
pub async fn write(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    value: Value,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request
            .header(header::COOKIE, format!("golf_session={token}"))
            .header("x-csrf-token", golf_api::auth::derive_csrf_token(token));
    }
    app.clone()
        .oneshot(request.body(Body::from(value.to_string())).unwrap())
        .await
        .unwrap()
}
