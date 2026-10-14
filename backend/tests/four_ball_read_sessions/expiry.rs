use super::support::*;
use axum::http::StatusCode;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_membership_wait_denies_both_four_ball_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let mut events = state.live_events.subscribe();
    let mut outcomes = Vec::new();
    for path in paths() {
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

async fn loading_expiry(pool: PgPool, confirmation: bool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (admin, _) = session(&pool).await;
    if confirmation {
        crate::four_ball_fixture::fill(&pool, admin).await;
        golf_api::repositories::four_ball::confirm(&pool, admin, ROUND, TEAM)
            .await
            .unwrap();
    }
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut outcomes = Vec::new();
    for path in paths() {
        let (session, token) = session(&pool).await;
        let baseline = body(get(&app, &path, &token).await).await;
        assert_eq!(baseline["holes"].as_array().unwrap().len(), 18);
        assert_eq!(baseline["holes_scored"], if confirmation { 18 } else { 0 });
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query(if confirmation {
            "LOCK TABLE scorecard_confirmations IN ACCESS EXCLUSIVE MODE"
        } else {
            "LOCK TABLE four_ball_inputs IN ACCESS EXCLUSIVE MODE"
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
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
    for (status, value) in outcomes {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(value["error"]["code"], "unauthenticated");
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_final_confirmation_loading_denies_both_reads(pool: PgPool) {
    loading_expiry(pool, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn empty_cards_expiring_during_input_loading_deny_both_reads(pool: PgPool) {
    loading_expiry(pool, false).await;
}
