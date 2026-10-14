use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};

#[sqlx::test(migrations = "../migrations")]
async fn revocation_before_transactional_authorization_denies_both_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
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
        // The request passed the extractor before waiting on transactional authority.
        lock_membership(&mut gate).await;
        let router = app.clone();
        let pending_path = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending_path, &token).await });
        wait_blocked(&pool, pid).await;
        sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
            .bind(session)
            .execute(&mut *gate)
            .await
            .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        let status = response.status();
        let cache = response.headers().get(header::CACHE_CONTROL).cloned();
        let value = body(response).await;
        eprintln!("{path}: status={status}, code={}", value["error"]["code"]);
        observed.push((path, status, cache, value));
    }
    assert!(events.try_recv().is_err());
    for (path, status, cache, value) in observed {
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
        assert!(cache.unwrap().to_str().unwrap().contains("no-store"));
        assert_eq!(value["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn credential_replacement_before_authority_denies_both_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
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
        sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
            .bind(format!("replacement-test-hash-{session}"))
            .bind(USER)
            .execute(&mut *gate)
            .await
            .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(10), read)
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
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn authorized_reads_serialize_before_later_revocation_and_credential_changes(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let mut events = state.live_events.subscribe();
    for credential_change in [false, true] {
        for path in paths() {
            let (session, token) = session(&pool).await;
            let before = body(get(&app, &path, &token).await).await;
            let mut gate = pool.begin().await.unwrap();
            let pid = blocker(&mut gate).await;
            lock_membership(&mut gate).await;
            let router = app.clone();
            let pending = path.clone();
            let pending_token = token.clone();
            let read = tokio::spawn(async move { get(&router, &pending, &pending_token).await });
            let reader_pid = wait_blocked(&pool, pid).await;
            let write_pool = pool.clone();
            let write = tokio::spawn(async move {
                if credential_change {
                    sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
                        .bind(format!("later-test-hash-{session}"))
                        .bind(USER)
                        .execute(&write_pool)
                        .await
                        .unwrap();
                } else {
                    golf_api::repositories::auth::revoke_session(&write_pool, session)
                        .await
                        .unwrap();
                }
            });
            wait_blocked(&pool, reader_pid).await;
            gate.commit().await.unwrap();
            let response = timeout(Duration::from_secs(10), read)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_eq!(body(response).await, before);
            timeout(Duration::from_secs(10), write)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                get(&app, &path, &token).await.status(),
                StatusCode::UNAUTHORIZED
            );
        }
    }
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn membership_removal_retries_with_fresh_authority_on_both_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    for path in paths() {
        let (_, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
            .bind(TRIP)
            .bind(USER)
            .execute(&mut *gate)
            .await
            .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert!(
            response.headers()[header::CACHE_CONTROL]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body(response).await["error"]["code"], "forbidden");
        sqlx::query(
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    }
}
