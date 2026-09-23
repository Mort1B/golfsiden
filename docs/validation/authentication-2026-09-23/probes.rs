use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    proxy::ProxyTrustConfig,
    rate_limit::{RateLimitRoute, RateLimiter},
    repositories::auth,
};
use sqlx::PgPool;
use std::time::{Duration, Instant};
use tower::ServiceExt;
use uuid::Uuid;

fn rate_capacity_probe() {
    let limiter = RateLimiter::production();
    let client = ProxyTrustConfig::direct().client_identity(&Default::default());
    let start = Instant::now();
    for user in 0u32..4 {
        for _ in 0..10 {
            assert!(
                limiter
                    .check(RateLimitRoute::Login, client, &user.to_be_bytes())
                    .is_ok()
            );
        }
    }
    assert!(
        limiter
            .check(RateLimitRoute::Login, client, &0u32.to_be_bytes())
            .is_err()
    );
    assert!(
        limiter
            .check(RateLimitRoute::Login, client, b"unused-client-cap-control")
            .is_err()
    );
    let mut admitted = 0;
    let mut denied = 0;
    for id in 0u128..8192 {
        if limiter
            .check(RateLimitRoute::RecoveryPreview, client, &id.to_be_bytes())
            .is_ok()
        {
            admitted += 1
        } else {
            denied += 1
        }
    }
    let reopened = limiter
        .check(RateLimitRoute::Login, client, &0u32.to_be_bytes())
        .is_ok();
    assert!(start.elapsed() < Duration::from_secs(60));
    assert!(reopened);
    println!(
        "AUTH-1: actual production limiter; login per-key and client limits initially denied; preview checks admitted={admitted}, denied={denied}; login accepted again before60s=true; elapsed_ms={}",
        start.elapsed().as_millis()
    );
}

async fn expiry_probe(pool: &PgPool, expire_while_waiting: bool) {
    let uid = Uuid::new_v4();
    let pid = Uuid::new_v4();
    let tid = Uuid::new_v4();
    let rid = Uuid::new_v4();
    sqlx::query("INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,'Synthetic review player',12)").bind(pid).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO users(id,username,display_name,role,player_id) VALUES($1,$2,'Synthetic reviewer','viewer',$3)").bind(uid).bind(format!("review_{}",&uid.simple().to_string()[..16])).bind(pid).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Synthetic access review',CURRENT_DATE,CURRENT_DATE,1)").bind(tid).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES($1,$2,12)").bind(tid).bind(pid).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(tid)
    .bind(uid)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES($1,$2,1,'Synthetic round',CURRENT_DATE,'TBD','TBD','individual_stroke_play')").bind(rid).bind(tid).execute(pool).await.unwrap();
    let token = golf_api::auth::generate_session_token().unwrap();
    let lifetime = if expire_while_waiting {
        chrono::Duration::seconds(3)
    } else {
        chrono::Duration::hours(1)
    };
    let principal = auth::create_session(
        pool,
        uid,
        &hash_session_token(&token),
        chrono::Utc::now() + lifetime,
    )
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tournament_handicap_history WHERE tournament_id=$1",
    )
    .bind(tid)
    .fetch_one(pool)
    .await
    .unwrap();
    let mut lock = pool.begin().await.unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR UPDATE")
        .bind(tid)
        .execute(&mut *lock)
        .await
        .unwrap();
    let req = Request::post(format!(
        "/api/tournaments/{tid}/players/{pid}/handicap-corrections"
    ))
    .header("cookie", format!("golf_session={token}"))
    .header("x-csrf-token", derive_csrf_token(&token))
    .header("content-type", "application/json")
    .body(Body::from(
        r#"{"handicap_index":5,"reason":"Synthetic boundary review"}"#,
    ))
    .unwrap();
    let task = tokio::spawn(app.clone().oneshot(req));
    tokio::time::timeout(Duration::from_secs(2),async {
  loop {
   let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) AND query LIKE 'SELECT id FROM tournaments%')").bind(blocker).fetch_one(pool).await.unwrap();
   if blocked {break;}tokio::time::sleep(Duration::from_millis(10)).await;
  }
 }).await.expect("request must reach parent-row wait before expiry");
    if expire_while_waiting {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let expired: bool = sqlx::query_scalar(
                    "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1",
                )
                .bind(principal.session_id)
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
    lock.commit().await.unwrap();
    let response = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let status = response.status();
    let handicap:f64=sqlx::query_scalar("SELECT tournament_handicap::float8 FROM tournament_players WHERE tournament_id=$1 AND player_id=$2").bind(tid).bind(pid).fetch_one(pool).await.unwrap();
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tournament_handicap_history WHERE tournament_id=$1",
    )
    .bind(tid)
    .fetch_one(pool)
    .await
    .unwrap();
    let event = events.try_recv().is_ok();
    let session_response = app
        .oneshot(
            Request::get("/api/auth/session")
                .header("cookie", format!("golf_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(handicap, 5.0);
    assert_eq!(after - before, 1);
    assert!(event);
    assert_eq!(
        session_response.status(),
        if expire_while_waiting {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::OK
        }
    );
    println!(
        "AUTH-2: expired_before_lock_release={expire_while_waiting}; correction_status={status}; handicap12_to5=true; audit_delta={}; invalidation={event}; subsequent_session_status={}",
        after - before,
        session_response.status()
    );
}
#[tokio::main]
async fn main() {
    rate_capacity_probe();
    let url = std::env::var("DATABASE_URL").unwrap();
    assert!(
        url.starts_with("postgres://golf_review:") && url.ends_with("@127.0.0.1:55443/golf_review"),
        "requires the designated disposable local database"
    );
    let pool = PgPool::connect(&url).await.unwrap();
    golf_api::schema::MIGRATOR.run(&pool).await.unwrap();
    expiry_probe(&pool, false).await;
    expiry_probe(&pool, true).await;
    pool.close().await;
}
