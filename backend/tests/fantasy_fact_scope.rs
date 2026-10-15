#![cfg(feature = "database-tests")]
mod fantasy_results_support;
use fantasy_results_support::*;

#[sqlx::test(migrations = "../migrations")]
async fn expiry_after_canonical_fact_wait_rolls_back_finalization(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE scores IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *holder)
        .await
        .unwrap();
    // Expiry is arranged before dispatch. The request holds session authority
    // locks while blocked on canonical facts, so do not edit that row afterward.
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '3 seconds' WHERE id=$1",
    )
    .bind(f.member_session)
    .execute(&pool)
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let path = format!(
        "/api/tournaments/{}/fantasy/rounds/{r}/results",
        f.base.tournament
    );
    let task = tokio::spawn(async move {
        app.oneshot(request("GET", &path, Some("fantasy-member"), json!(null)))
            .await
            .unwrap()
    });
    wait_for_lock(&pool, "%WITH target AS%").await;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let expired: bool = sqlx::query_scalar(
                "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1",
            )
            .bind(f.member_session)
            .fetch_one(&pool)
            .await
            .unwrap();
            if expired {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    holder.rollback().await.unwrap();
    let response = task.await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM fantasy_rounds WHERE locked_at IS NOT NULL"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_selections")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_lineups")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(events.try_recv().is_err());
}
