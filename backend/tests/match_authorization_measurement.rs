#![cfg(feature = "database-tests")]
// Ignored by ordinary validation: requires its own pg_stat_statements instance.
#[path = "match_authorization_measurement/fixture.rs"]
mod fixture;
#[path = "match_authorization_measurement/measure.rs"]
mod measure;
use sqlx::PgPool;

#[sqlx::test(migrations = "../migrations")]
#[ignore = "exclusive disposable PostgreSQL with pg_stat_statements required"]
async fn twelve_matches(pool: PgPool) {
    measure::run(pool, 12).await;
}
#[sqlx::test(migrations = "../migrations")]
#[ignore = "exclusive disposable PostgreSQL with pg_stat_statements required"]
async fn twenty_four_matches(pool: PgPool) {
    measure::run(pool, 24).await;
}
#[sqlx::test(migrations = "../migrations")]
#[ignore = "exclusive disposable PostgreSQL with pg_stat_statements required"]
async fn hundred_matches(pool: PgPool) {
    measure::run(pool, 100).await;
}
