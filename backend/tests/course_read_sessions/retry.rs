use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;

async fn failure(pool: PgPool, code: &str) {
    let (_, app) = fixture(&pool).await;
    let (_, token) = session(&pool).await;
    sqlx::raw_sql("CREATE SEQUENCE course_read_attempts; CREATE TABLE course_read_failure(code text NOT NULL); CREATE FUNCTION fail_course_read() RETURNS boolean LANGUAGE plpgsql AS $$ DECLARE failure_code text; BEGIN PERFORM nextval('course_read_attempts'); SELECT code INTO failure_code FROM course_read_failure; RAISE EXCEPTION 'synthetic course read failure' USING ERRCODE=failure_code; END $$; ALTER TABLE tournaments RENAME TO stored_tournaments; CREATE VIEW tournaments AS SELECT * FROM stored_tournaments WHERE fail_course_read();").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO course_read_failure VALUES($1)")
        .bind(code)
        .execute(&pool)
        .await
        .unwrap();
    for path in paths() {
        sqlx::query("ALTER SEQUENCE course_read_attempts RESTART WITH 1")
            .execute(&pool)
            .await
            .unwrap();
        let r = get(&app, &path, &token).await;
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        assert_eq!(
            r.status(),
            if code == "40001" {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        );
        assert_eq!(
            body(r).await["error"]["code"],
            if code == "40001" {
                "service_unavailable"
            } else {
                "internal_error"
            }
        );
        let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM course_read_attempts")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(attempts, if code == "40001" { 2 } else { 1 });
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn serialization_failure_retries_once_then_private_503(pool: PgPool) {
    failure(pool, "40001").await;
}
#[sqlx::test(migrations = "../migrations")]
async fn unrelated_database_failure_is_not_retried(pool: PgPool) {
    failure(pool, "XX000").await;
}
