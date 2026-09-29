#![cfg(feature = "database-tests")]
#[path = "tournament_details_support/mod.rs"]
mod support;
use support::*;
use tokio::time::{Duration as Wait, sleep, timeout};
async fn wait_blocked(pool: &PgPool, pid: i32) {
    timeout(Wait::from_secs(8), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))").bind(pid).fetch_one(pool).await.unwrap();
            if blocked { break; }
            sleep(Wait::from_millis(10)).await;
        }
    }).await.expect("request reached lock wait");
}
#[sqlx::test(migrations = "../migrations")]
async fn revocation_while_waiting_on_round_is_rejected_without_event(pool: PgPool) {
    seed(&pool).await;
    let data = payload(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let mut gate = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM rounds WHERE tournament_id=$1 FOR UPDATE")
        .bind(TRIP)
        .fetch_all(&mut *gate)
        .await
        .unwrap();
    let task = tokio::spawn(api::router(Arc::clone(&state)).oneshot(request(TOKEN, data)));
    wait_blocked(&pool, pid).await;
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(TRIP)
        .bind(ADMIN)
        .execute(&pool)
        .await
        .unwrap();
    gate.commit().await.unwrap();
    assert_eq!(task.await.unwrap().unwrap().status(), StatusCode::FORBIDDEN);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT name FROM tournaments WHERE id=$1")
            .bind(TRIP)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Original"
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_after_write_wait_rolls_back_without_event(pool: PgPool) {
    seed(&pool).await;
    sqlx::raw_sql("CREATE FUNCTION hold_details_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(330033); RETURN NEW; END $$; CREATE TRIGGER hold_details_test AFTER UPDATE ON tournaments FOR EACH ROW EXECUTE FUNCTION hold_details_test();").execute(&pool).await.unwrap();
    let mut gate = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(330033)")
        .execute(&mut *gate)
        .await
        .unwrap();
    sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '3 seconds' WHERE user_id=$1").bind(ADMIN).execute(&pool).await.unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let data = payload(&pool).await;
    let task = tokio::spawn(api::router(Arc::clone(&state)).oneshot(request(TOKEN, data)));
    wait_blocked(&pool, pid).await;
    loop {
        let expired: bool = sqlx::query_scalar(
            "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE user_id=$1",
        )
        .bind(ADMIN)
        .fetch_one(&pool)
        .await
        .unwrap();
        if expired {
            break;
        }
        sleep(Wait::from_millis(25)).await;
    }
    gate.commit().await.unwrap();
    assert_eq!(
        task.await.unwrap().unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT name FROM tournaments WHERE id=$1")
            .bind(TRIP)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Original"
    );
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}
#[sqlx::test(migrations = "../migrations")]
async fn snapshot_isolation_cannot_hide_committed_round_date_changes(pool: PgPool) {
    seed(&pool).await;
    let mut stale = pool.begin().await.unwrap();
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *stale)
        .await
        .unwrap();
    sqlx::query("SELECT * FROM rounds")
        .fetch_all(&mut *stale)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET round_date='2026-09-03' WHERE tournament_id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.tournament_details_id',$1,true),set_config('app.tournament_details_session_id',(SELECT id::text FROM user_sessions WHERE user_id=$2),true)").bind(TRIP.to_string()).bind(ADMIN).execute(&mut *stale).await.unwrap();
    let error = sqlx::query("UPDATE tournaments SET end_date='2026-09-02' WHERE id=$1")
        .bind(TRIP)
        .execute(&mut *stale)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().constraint(),
        Some("tournament_details_isolation")
    );
    stale.rollback().await.unwrap();
}
