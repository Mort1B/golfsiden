use super::support::*;
use axum::http::StatusCode;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
async fn expiry_at_wait(pool: PgPool, metadata: bool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let mut events = state.live_events.subscribe();
    let mut outcomes = Vec::new();
    for path in [path()] {
        let (session, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        if metadata {
            sqlx::query("LOCK TABLE tournaments IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *gate)
                .await
                .unwrap();
        } else {
            lock_membership(&mut gate).await;
        }
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1").bind(session).execute(&pool).await.unwrap();
        let router = app.clone();
        let p = path.clone();
        let read = tokio::spawn(async move { get(&router, &p, &token).await });
        expire_at_wait(&pool, session, pid).await;
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_private(&r);
        let status = r.status();
        let value = body(r).await;
        eprintln!("{path}: status={status}, code={}", value["error"]["code"]);
        outcomes.push((status, value));
    }
    assert!(events.try_recv().is_err());
    for (status, value) in outcomes {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(value["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_membership_wait_is_denied(pool: PgPool) {
    expiry_at_wait(pool, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_metadata_lookup_is_denied(pool: PgPool) {
    expiry_at_wait(pool, true).await;
}
