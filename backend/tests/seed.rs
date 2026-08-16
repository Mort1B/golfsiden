#![cfg(feature = "database-tests")]

use sqlx::PgPool;

#[sqlx::test(migrations = "../migrations")]
async fn development_seed_is_username_only_and_idempotent(pool: PgPool) {
    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../seed.sql"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let usernames = sqlx::query_scalar::<_, String>("SELECT username FROM users ORDER BY username")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        usernames,
        vec![
            "admin",
            "anders",
            "bjarne",
            "christian",
            "daniel",
            "eirik",
            "fredrik",
            "geir",
            "henrik",
        ]
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap(),
        9
    );
}
