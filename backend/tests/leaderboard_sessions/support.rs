use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{
    AppState, api,
    auth::hash_session_token,
    domain::scorecards::ScoreOwner,
    repositories::{auth, round_lifecycle, scorecards, tournaments},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

pub const USER: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
pub const TRIP: Uuid = uuid!("00000000-0000-0000-0000-000000002001");
pub const ROUND: Uuid = uuid!("00000000-0000-0000-0000-000000004001");

pub struct Fixture {
    pub state: Arc<AppState>,
    pub app: Router,
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

pub async fn fixture(pool: &PgPool) -> Fixture {
    sqlx::raw_sql(include_str!("../../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
    let (session, _) = session(pool).await;
    let updated = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(pool)
        .await
        .unwrap();
    tournaments::start_authorized(pool, session, TRIP, updated)
        .await
        .unwrap();
    round_lifecycle::open_authorized(pool, session, ROUND)
        .await
        .unwrap();
    scorecards::save_authenticated(
        pool,
        scorecards::AuthenticatedSaveScore {
            session_id: session,
            round_id: ROUND,
            owner: ScoreOwner::Team {
                id: uuid!("00000000-0000-0000-0000-000000005001"),
            },
            hole_id: uuid!("00000000-0000-0000-0000-000000003201"),
            gross_strokes: 4,
        },
    )
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));
    Fixture { state, app }
}

pub fn paths() -> Vec<String> {
    [
        format!("/api/rounds/{ROUND}"),
        format!("/api/tournaments/{TRIP}"),
    ]
    .into_iter()
    .flat_map(|base| {
        [
            format!("{base}/leaderboards/gross"),
            format!("{base}/leaderboards/net"),
        ]
    })
    .collect()
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
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let pid: Option<i32> = sqlx::query_scalar("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) ORDER BY pid LIMIT 1")
                .bind(blocker).fetch_optional(pool).await.unwrap();
            if let Some(pid) = pid { return pid; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("read must reach the controlled database wait")
}

pub async fn lock_membership(tx: &mut Transaction<'_, Postgres>) {
    sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE")
        .bind(TRIP).bind(USER).execute(&mut **tx).await.unwrap();
}
