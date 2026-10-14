use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;
use tokio::time::{Duration, sleep, timeout};

async fn expire_during_wait(pool: PgPool, loading: bool, empty: bool, singles: bool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let mut events = state.live_events.subscribe();
    let targets = if singles {
        sqlx::query("UPDATE rounds SET scoring_format='singles_match_play' WHERE id=$1")
            .bind(ROUND)
            .execute(&pool)
            .await
            .unwrap();
        vec![format!("/api/rounds/{ROUND}/pairing-validation")]
    } else if empty {
        let (_, round) = empty_round(&pool).await;
        vec![
            format!("/api/rounds/{round}/pairing-validation"),
            format!("/api/rounds/{round}/completion-validation"),
        ]
    } else {
        paths()
    };
    for path in targets {
        let (session, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        if loading {
            if singles {
                sqlx::query("LOCK TABLE singles_matches IN ACCESS EXCLUSIVE MODE")
                    .execute(&mut *gate)
                    .await
                    .unwrap();
            } else {
                lock_loading(&mut gate, &path, empty).await;
            }
        } else {
            lock_membership(&mut gate).await;
        }
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
        assert!(active, "must observe a real wait before expiry: {path}");
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
async fn expiry_during_membership_wait_denies_both_reads(pool: PgPool) {
    expire_during_wait(pool, false, false, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_late_pairing_or_completion_loading_denies_reads(pool: PgPool) {
    expire_during_wait(pool, true, false, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn empty_readiness_results_still_recheck_expiry(pool: PgPool) {
    expire_during_wait(pool, true, true, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_singles_assignment_loading_denies_pairing_readiness(pool: PgPool) {
    expire_during_wait(pool, true, false, true).await;
}
