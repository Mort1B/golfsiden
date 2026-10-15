use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_membership_wait_denies_all_three_gets(pool: PgPool) {
    let (state, app) = fixture(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut observed = Vec::new();
    for path in paths() {
        let (sid, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        short_session(&pool, sid).await;
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        wait_expired(&pool, sid).await;
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        let status = r.status();
        let cache = r.headers().get(header::CACHE_CONTROL).cloned();
        let value = body(r).await;
        eprintln!("{path}: {status}, code={}", value["error"]["code"]);
        observed.push((status, cache, value));
    }
    assert!(events.try_recv().is_err());
    for (status, cache, value) in observed {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(cache.unwrap(), "private, no-store");
        assert_eq!(value["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn preset_final_loading_expiry_denies_populated_and_empty_results(pool: PgPool) {
    let (_, app) = fixture(&pool).await;
    for empty in [false, true] {
        if empty {
            sqlx::query("DELETE FROM course_presets")
                .execute(&pool)
                .await
                .unwrap();
        }
        let (sid, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query("LOCK TABLE courses IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *gate)
            .await
            .unwrap();
        short_session(&pool, sid).await;
        let router = app.clone();
        let read = tokio::spawn(async move {
            get(
                &router,
                &format!("/api/tournaments/{TRIP}/course-presets"),
                &token,
            )
            .await
        });
        wait_blocked(&pool, pid).await;
        wait_expired(&pool, sid).await;
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    }
}
