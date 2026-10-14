use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;
use tokio::time::{Duration, sleep, timeout};

async fn expire_during_wait(pool: PgPool, table_wait: bool, empty: bool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let mut events = state.live_events.subscribe();
    if empty {
        sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
            .bind(USER)
            .execute(&pool)
            .await
            .unwrap();
    }
    for path in paths()
        .into_iter()
        .filter(|p| !empty || !p.ends_with(&TRIP.to_string()))
    {
        let (session, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        if table_wait {
            lock_tournaments(&mut gate).await;
        } else {
            lock_membership(&mut gate).await;
        }
        // Generous pre-gate window; database observation proves a real wait
        // begins before expiry. No test clock or production timing hook.
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1").bind(session).execute(&pool).await.unwrap();
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        let active: bool = sqlx::query_scalar(
            "SELECT expires_at>clock_timestamp() FROM user_sessions WHERE id=$1",
        )
        .bind(session)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(active, "must observe controlled wait before expiry: {path}");
        timeout(Duration::from_secs(15), async {
            loop {
                let expired: bool = sqlx::query_scalar(
                    "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1",
                )
                .bind(session)
                .fetch_one(&pool)
                .await
                .unwrap();
                if expired {
                    break;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        assert_eq!(body(response).await["error"]["code"], "unauthenticated");
    }
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_membership_wait_denies_all_three_reads(pool: PgPool) {
    expire_during_wait(pool, false, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_tournament_loading_denies_all_three_reads(pool: PgPool) {
    expire_during_wait(pool, true, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn empty_collections_recheck_expiry_after_loading_wait(pool: PgPool) {
    expire_during_wait(pool, true, true).await;
}
