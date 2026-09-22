#![cfg(feature = "database-tests")]
#[path = "transaction_cancellation/proxy.rs"]
mod proxy;
use sqlx::PgPool;
use std::time::Duration;

async fn pid(pool: &PgPool) -> i32 {
    sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn ordinary_begin_cancellation_returns_clean_connection(source: PgPool) {
    let (proxy, pool) = proxy::Proxy::pool(&source).await;
    let before = pid(&pool).await;
    proxy.gate.arm();
    let begin = tokio::spawn({
        let pool = pool.clone();
        async move { pool.begin().await }
    });
    tokio::time::timeout(Duration::from_secs(5), proxy.gate.reached.notified())
        .await
        .unwrap();
    // The server accepted an unmodified BEGIN, but SQLx has not seen its reply.
    begin.abort();
    assert!(begin.await.unwrap_err().is_cancelled());
    proxy.gate.release.notify_one();
    assert_eq!(pid(&pool).await, before, "exercise actual connection reuse");
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await
        .unwrap();
    let isolation: String = sqlx::query_scalar("SHOW transaction_isolation")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(isolation, "repeatable read");
    tx.commit().await.unwrap();
    assert_eq!(
        proxy
            .gate
            .commands()
            .iter()
            .filter(|c| *c == "ROLLBACK")
            .count(),
        1
    );
    pool.close().await;
}

#[sqlx::test]
async fn cancelled_savepoint_preserves_outer_transaction(source: PgPool) {
    use sqlx::Acquire;
    let (proxy, pool) = proxy::Proxy::pool(&source).await;
    sqlx::query("CREATE TABLE cancellation_marks (id integer PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    let mut outer = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO cancellation_marks VALUES (1)")
        .execute(&mut *outer)
        .await
        .unwrap();
    proxy.gate.arm();
    {
        let begin = outer.begin();
        tokio::pin!(begin);
        tokio::select! {
            biased;
            result = &mut begin => panic!("savepoint unexpectedly finished: {result:?}"),
            held = tokio::time::timeout(Duration::from_secs(5), proxy.gate.reached.notified()) => { held.unwrap(); },
        }
    }
    proxy.gate.release.notify_one();
    sqlx::query("INSERT INTO cancellation_marks VALUES (2)")
        .execute(&mut *outer)
        .await
        .unwrap();
    outer.commit().await.unwrap();
    let ids: Vec<i32> = sqlx::query_scalar("SELECT id FROM cancellation_marks ORDER BY id")
        .fetch_all(&source)
        .await
        .unwrap();
    assert_eq!(ids, vec![1, 2]);
    assert_eq!(
        proxy
            .gate
            .commands()
            .iter()
            .filter(|c| *c == "ROLLBACK")
            .count(),
        1
    );
    pool.close().await;
}

#[sqlx::test]
async fn cancelled_request_work_rolls_back_writes_settings_and_locks(source: PgPool) {
    let (_proxy, pool) = proxy::Proxy::pool(&source).await;
    sqlx::query("CREATE TABLE cancellation_marks (id integer PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    let before = pid(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('golf.test_actor', 'account-a', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(713829)")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO cancellation_marks VALUES (1)")
        .execute(&mut *tx)
        .await
        .unwrap();
    drop(tx);
    assert_eq!(pid(&pool).await, before);
    let setting: Option<String> =
        sqlx::query_scalar("SELECT NULLIF(current_setting('golf.test_actor', true), '')")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(setting, None);
    let mut other = source.begin().await.unwrap();
    let unlocked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(713829)")
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(unlocked);
    other.rollback().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM cancellation_marks")
        .fetch_one(&source)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("INSERT INTO cancellation_marks VALUES (2)")
        .execute(&pool)
        .await
        .unwrap();
    let ids: Vec<i32> = sqlx::query_scalar("SELECT id FROM cancellation_marks")
        .fetch_all(&source)
        .await
        .unwrap();
    assert_eq!(ids, vec![2], "next request autocommits independently");
    pool.close().await;
}

#[sqlx::test]
async fn failed_and_successful_begins_preserve_modes_and_reuse(source: PgPool) {
    let (_proxy, pool) = proxy::Proxy::pool(&source).await;
    let before = pid(&pool).await;
    assert!(matches!(
        pool.begin_with("SELECT 1").await,
        Err(sqlx::Error::BeginFailed)
    ));
    assert!(pool.begin_with("INVALID BEGIN").await.is_err());
    assert_eq!(pid(&pool).await, before);
    let mut tx = pool
        .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .await
        .unwrap();
    let isolation: String = sqlx::query_scalar("SHOW transaction_isolation")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let read_only: String = sqlx::query_scalar("SHOW transaction_read_only")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(isolation, "repeatable read");
    assert_eq!(read_only, "on");
    tx.commit().await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    let read_only: String = sqlx::query_scalar("SHOW transaction_read_only")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(read_only, "off");
    tx.rollback().await.unwrap();
    pool.close().await;
}

#[sqlx::test]
async fn successful_pool_reuse_adds_no_rollback_queries(source: PgPool) {
    let (proxy, pool) = proxy::Proxy::pool(&source).await;
    let before = pid(&pool).await;
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT 1").execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
    }
    assert_eq!(pid(&pool).await, before);
    let commands = proxy.gate.commands();
    assert_eq!(commands.iter().filter(|c| *c == "BEGIN").count(), 100);
    assert_eq!(commands.iter().filter(|c| *c == "COMMIT").count(), 100);
    assert_eq!(commands.iter().filter(|c| *c == "ROLLBACK").count(), 0);
    println!(
        "100 reused-connection transactions: {:?}; 100 BEGIN, 100 COMMIT, 0 ROLLBACK",
        start.elapsed()
    );
    pool.close().await;
}
