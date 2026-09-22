#[tokio::test]
async fn cancellation_before_transaction_depth_is_recorded() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        pool.begin_with("BEGIN; SELECT pg_sleep(0.4)"),
    )
    .await;
    assert!(result.is_err());
    sqlx::query("SELECT 1").execute(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    let error = sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("25001")
    );
    tx.rollback().await.unwrap();
    pool.close().await;
}
