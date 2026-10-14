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

pub const TEAM: Uuid = uuid!("00000000-0000-0000-0000-000000005001");
pub const DRAFT: Uuid = uuid!("00000000-0000-0000-0000-000000004002");
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
    let (session, _) = session(pool).await;
    open_final(pool, session).await;
    golf_api::repositories::round_lifecycle::open_authorized(pool, session, ROUND)
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
        format!("/api/rounds/{ROUND}/score-access"),
        format!("/api/rounds/{ROUND}/scorecards/team/{TEAM}"),
        format!("/api/rounds/{ROUND}/scorecards/team/{TEAM}/scoring"),
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

pub async fn lock_loading(tx: &mut Transaction<'_, Postgres>, path: &str) {
    let sql = if path.ends_with("/score-access") {
        "LOCK TABLE team_memberships IN ACCESS EXCLUSIVE MODE"
    } else {
        // Last card query, after handicap, hole and score loading.
        "LOCK TABLE scorecard_confirmations IN ACCESS EXCLUSIVE MODE"
    };
    sqlx::query(sql).execute(&mut **tx).await.unwrap();
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
      'scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scores s),
      'audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM score_audits a),
      'confirmations',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM scorecard_confirmations c),
      'handicaps',(SELECT jsonb_agg(to_jsonb(s) ORDER BY round_id,player_id) FROM round_handicap_snapshots s),
      'team_handicaps',(SELECT jsonb_agg(to_jsonb(s) ORDER BY round_id,team_id) FROM round_team_handicap_snapshots s)
    )").fetch_one(pool).await.unwrap()
}

pub async fn fill_team(pool: &PgPool) {
    use golf_api::{domain::scorecards::ScoreOwner, repositories::scorecards};
    let holes:Vec<Uuid>=sqlx::query_scalar("SELECT h.id FROM holes h JOIN rounds r ON r.tee_id=h.tee_id WHERE r.id=$1 ORDER BY h.hole_number").bind(ROUND).fetch_all(pool).await.unwrap();
    for hole in holes {
        scorecards::save(
            pool,
            scorecards::SaveScore {
                round_id: ROUND,
                hole_id: hole,
                owner: ScoreOwner::Team { id: TEAM },
                gross_strokes: 4,
                submitted_by: USER,
            },
        )
        .await
        .unwrap();
    }
    scorecards::confirm(pool, ROUND, ScoreOwner::Team { id: TEAM }, USER)
        .await
        .unwrap();
}

pub async fn session_for(pool: &PgPool, user: Uuid) -> (Uuid, String) {
    let token = Uuid::new_v4().to_string();
    let p = auth::create_session(
        pool,
        user,
        &hash_session_token(&token),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
    (p.session_id, token)
}
