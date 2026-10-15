use super::support::*;
use axum::http::StatusCode;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_membership_wait_is_denied(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let mut events = state.live_events.subscribe();
    let mut outcomes = Vec::new();
    for path in [path()] {
        let (session, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
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

async fn loading_expiry(pool: PgPool, empty: bool, counts: bool) {
    let state = fixture(&pool).await;
    if !empty {
        invitation(&pool).await;
    }
    let app = app(&state);
    let before = read_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut outcomes = Vec::new();
    for path in [path()] {
        let (session, token) = session(&pool).await;
        let baseline = get(&app, &path, &token).await;
        assert_eq!(baseline.status(), StatusCode::OK);
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query(if counts {
            "LOCK TABLE invitation_redemptions IN ACCESS EXCLUSIVE MODE"
        } else {
            "LOCK TABLE tournament_invitations IN ACCESS EXCLUSIVE MODE"
        })
        .execute(&mut *gate)
        .await
        .unwrap();
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
    assert_eq!(read_state(&pool).await, before);
    assert!(events.try_recv().is_err());
    for (status, value) in outcomes {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(value["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_final_metadata_loading_is_denied(pool: PgPool) {
    loading_expiry(pool, false, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_redemption_count_loading_is_denied(pool: PgPool) {
    loading_expiry(pool, false, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn empty_list_still_rechecks_expiry_after_metadata_wait(pool: PgPool) {
    loading_expiry(pool, true, false).await;
}
