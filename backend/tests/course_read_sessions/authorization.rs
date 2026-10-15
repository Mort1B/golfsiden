use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};

async fn lost_before_authority(pool: PgPool, credential_change: bool) {
    let (state, app) = fixture(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut observed = Vec::new();
    for path in paths() {
        let (session, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query("SELECT id FROM user_sessions WHERE id=$1 FOR UPDATE")
            .bind(session)
            .execute(&mut *gate)
            .await
            .unwrap();
        sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
            .bind(USER)
            .execute(&mut *gate)
            .await
            .unwrap();
        lock_membership(&mut gate).await;
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        if credential_change {
            sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
                .bind(format!("synthetic-replacement-{session}"))
                .bind(USER)
                .execute(&mut *gate)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
                .bind(session)
                .execute(&mut *gate)
                .await
                .unwrap();
        }
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        let status = response.status();
        let cache = response.headers().get(header::CACHE_CONTROL).cloned();
        let value = body(response).await;
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
async fn revocation_after_extraction_before_authority_denies_all_three_gets(pool: PgPool) {
    lost_before_authority(pool, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn credential_replacement_after_extraction_denies_all_three_gets(pool: PgPool) {
    lost_before_authority(pool, true).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn membership_demotion_retries_exact_admin_and_cancellation_releases_session(pool: PgPool) {
    let (_, app) = fixture(&pool).await;
    for path in paths() {
        let (_, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        sqlx::query(
            "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&mut *gate)
        .await
        .unwrap();
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_eq!(body(r).await["error"]["code"], "forbidden");
        sqlx::query(
            "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let (sid, token) = session(&pool).await;
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        read.abort();
        assert!(read.await.unwrap_err().is_cancelled());
        gate.commit().await.unwrap();
        timeout(
            Duration::from_secs(10),
            golf_api::repositories::auth::revoke_session(&pool, sid),
        )
        .await
        .unwrap()
        .unwrap();
    }
    // A conflict that retains admin must reload facts from the new snapshot.
    let (_, token) = session(&pool).await;
    let path = paths()[1].clone();
    let before = body(get(&app, &path, &token).await).await;
    let mut gate = pool.begin().await.unwrap();
    let pid = blocker(&mut gate).await;
    lock_membership(&mut gate).await;
    let router = app.clone();
    let pending = path.clone();
    let read_token = token.clone();
    let read = tokio::spawn(async move { get(&router, &pending, &read_token).await });
    wait_blocked(&pool, pid).await;
    sqlx::query(
        "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
    )
    .bind(TRIP)
    .bind(USER)
    .execute(&mut *gate)
    .await
    .unwrap();
    sqlx::query("UPDATE course_presets SET display_order=4 WHERE display_order=1")
        .execute(&mut *gate)
        .await
        .unwrap();
    gate.commit().await.unwrap();
    let r = timeout(Duration::from_secs(10), read)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let value = body(r).await;
    assert_ne!(value, before);
    assert_eq!(value, body(get(&app, &path, &token).await).await);
}
