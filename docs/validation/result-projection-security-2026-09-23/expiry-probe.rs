use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, generate_session_token, hash_session_token},
    repositories::auth,
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

async fn check(pool: &PgPool, expire: bool) {
    let uid = Uuid::new_v4();
    let tid = Uuid::new_v4();
    let rid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users(id,username,display_name,role) VALUES($1,$2,'Synthetic admin','viewer')",
    )
    .bind(uid)
    .bind(format!("review_{}", &uid.simple().to_string()[..16]))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES($1,'Synthetic share review',CURRENT_DATE,CURRENT_DATE,1,1)").bind(tid).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(tid)
    .bind(uid)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES($1,$2,1,'Synthetic round',CURRENT_DATE,'TBD','TBD','individual_stroke_play')").bind(rid).bind(tid).execute(pool).await.unwrap();
    let token = generate_session_token().unwrap();
    let life = if expire {
        chrono::Duration::seconds(3)
    } else {
        chrono::Duration::hours(1)
    };
    let principal = auth::create_session(
        pool,
        uid,
        &hash_session_token(&token),
        chrono::Utc::now() + life,
    )
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let mut lock = pool.begin().await.unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE tournament_result_share_audits IN SHARE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let req = Request::post(format!("/api/tournaments/{tid}/result-share"))
        .header("cookie", format!("golf_session={token}"))
        .header("x-csrf-token", derive_csrf_token(&token))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"expected_grant_id":null}"#))
        .unwrap();
    let task = tokio::spawn(app.clone().oneshot(req));
    tokio::time::timeout(Duration::from_secs(2),async {
  loop {
   let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks l JOIN pg_stat_activity a ON a.pid=l.pid WHERE a.datname=current_database() AND l.relation='tournament_result_share_audits'::regclass AND l.mode='RowExclusiveLock' AND NOT l.granted AND $1=ANY(pg_blocking_pids(a.pid)))").bind(blocker).fetch_one(pool).await.unwrap();
   if waiting {break;}tokio::time::sleep(Duration::from_millis(10)).await;
  }
 }).await.expect("must observe late audit-table wait");
    assert!(!expired(pool, principal.session_id).await);
    if expire {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !expired(pool, principal.session_id).await {
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
    let body: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let grants:i64=sqlx::query_scalar("SELECT count(*) FROM tournament_result_shares WHERE tournament_id=$1 AND revoked_at IS NULL").bind(tid).fetch_one(pool).await.unwrap();
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tournament_result_share_audits WHERE tournament_id=$1",
    )
    .bind(tid)
    .fetch_one(pool)
    .await
    .unwrap();
    let event = events.try_recv().unwrap();
    assert_eq!(event.tournament_id, tid);
    let session_status = app
        .clone()
        .oneshot(
            Request::get("/api/auth/session")
                .header("cookie", format!("golf_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status();
    let public_req = Request::post(format!(
        "/api/public/results/{}",
        body["grant"]["id"].as_str().unwrap()
    ))
    .header("content-type", "application/json")
    .body(Body::from(
        serde_json::json!({"token":body["token"],"metric":"gross"}).to_string(),
    ))
    .unwrap();
    let public_status = app.oneshot(public_req).await.unwrap().status();
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!((grants, audits), (1, 1));
    assert_eq!(public_status, StatusCode::OK);
    assert_eq!(
        session_status,
        if expire {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::OK
        }
    );
    println!(
        "SHARE-1: expired_before_audit_unlock={expire}; issue={status}; live_grants={grants}; audits={audits}; matching_event=true; next_session={session_status}; anonymous_read={public_status}"
    );
}
async fn expired(pool: &PgPool, session: Uuid) -> bool {
    sqlx::query_scalar("SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1")
        .bind(session)
        .fetch_one(pool)
        .await
        .unwrap()
}
#[tokio::main]
async fn main() {
    let url = std::env::var("DATABASE_URL").unwrap();
    assert!(
        url.starts_with("postgres://golf_review:") && url.ends_with("@127.0.0.1:55443/golf_review")
    );
    let pool = PgPool::connect(&url).await.unwrap();
    golf_api::schema::MIGRATOR.run(&pool).await.unwrap();
    check(&pool, false).await;
    check(&pool, true).await;
    pool.close().await;
}
