use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};

#[sqlx::test(migrations = "../migrations")]
async fn revocation_before_transactional_authorization_denies_all_boards(pool: PgPool) {
    let f = fixture(&pool).await;
    let mut events = f.state.live_events.subscribe();
    for path in paths() {
        let (session, token) = session(&pool).await;
        let baseline = get(&f.app, &path, &token).await;
        assert_eq!(baseline.status(), StatusCode::OK);
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        // Old code waits only on membership; repaired code waits on session.
        // The extractor's nonlocking lookup still succeeds in both versions.
        sqlx::query("SELECT id FROM user_sessions WHERE id=$1 FOR UPDATE")
            .bind(session)
            .execute(&mut *gate)
            .await
            .unwrap();
        lock_membership(&mut gate).await;
        let app = f.app.clone();
        let pending_path = path.clone();
        let pending_token = token.clone();
        let task = tokio::spawn(async move { get(&app, &pending_path, &pending_token).await });
        wait_blocked(&pool, pid).await;
        sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
            .bind(session)
            .execute(&mut *gate)
            .await
            .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
        assert!(
            response.headers()[header::CACHE_CONTROL]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
        assert_eq!(body(response).await["error"]["code"], "unauthenticated");
        assert_eq!(
            get(&f.app, &path, &token).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn credential_change_before_transactional_authorization_denies_all_boards(pool: PgPool) {
    let f = fixture(&pool).await;
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
        let app = f.app.clone();
        let pending_path = path.clone();
        let task = tokio::spawn(async move { get(&app, &pending_path, &token).await });
        wait_blocked(&pool, pid).await;
        sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
            .bind(format!("changed-test-hash-{session}"))
            .bind(USER)
            .execute(&mut *gate)
            .await
            .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(body(response).await["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn authorized_read_serializes_before_later_revocation(pool: PgPool) {
    let f = fixture(&pool).await;
    for path in paths() {
        let (session, token) = session(&pool).await;
        let before = body(get(&f.app, &path, &token).await).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let app = f.app.clone();
        let pending_path = path.clone();
        let pending_token = token.clone();
        let read = tokio::spawn(async move { get(&app, &pending_path, &pending_token).await });
        let reader_pid = wait_blocked(&pool, pid).await;
        let revoke_pool = pool.clone();
        let revoke = tokio::spawn(async move {
            golf_api::repositories::auth::revoke_session(&revoke_pool, session)
                .await
                .unwrap();
        });
        wait_blocked(&pool, reader_pid).await;
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(body(response).await, before);
        timeout(Duration::from_secs(5), revoke)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            get(&f.app, &path, &token).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn membership_removed_during_wait_is_reauthorized_after_serialization_failure(pool: PgPool) {
    let f = fixture(&pool).await;
    let (_, token) = session(&pool).await;
    let mut gate = pool.begin().await.unwrap();
    let pid = blocker(&mut gate).await;
    lock_membership(&mut gate).await;
    let app = f.app.clone();
    let path = paths().remove(0);
    let task = tokio::spawn(async move { get(&app, &path, &token).await });
    wait_blocked(&pool, pid).await;
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(TRIP)
        .bind(USER)
        .execute(&mut *gate)
        .await
        .unwrap();
    gate.commit().await.unwrap();
    let response = timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(body(response).await["error"]["code"], "forbidden");
}
