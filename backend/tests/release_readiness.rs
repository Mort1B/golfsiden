#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{
    AppState, api,
    schema::{self, RuntimeAuthorityError, SchemaCompatibilityError},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn current_schema_and_readiness_are_healthy(pool: PgPool) {
    schema::check_compatibility(&pool).await.unwrap();
    let response = api::router(AppState::new(pool))
        .oneshot(Request::get("/api/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, json!({"status":"ready"}));
}

#[sqlx::test(migrations = false)]
async fn missing_migration_history_is_not_compatible(pool: PgPool) {
    assert!(matches!(
        schema::check_compatibility(&pool).await,
        Err(SchemaCompatibilityError::MissingHistory)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn runtime_authority_rejects_an_unexpected_or_privileged_role(pool: PgPool) {
    assert!(matches!(
        schema::check_runtime_authority(&pool, "definitely_not_the_current_role").await,
        Err(RuntimeAuthorityError::UnexpectedRole)
    ));
    let current_user = sqlx::query_scalar::<_, String>("SELECT current_user")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(matches!(
        schema::check_runtime_authority(&pool, &current_user).await,
        Err(RuntimeAuthorityError::ExcessPrivileges)
            | Err(RuntimeAuthorityError::MigrationHistoryWritable)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn pending_and_unknown_migrations_are_rejected(pool: PgPool) {
    let latest = sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = $1")
        .bind(latest)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        schema::check_compatibility(&pool).await,
        Err(SchemaCompatibilityError::Pending(version)) if version == latest
    ));

    sqlx::query(
        "INSERT INTO _sqlx_migrations
           (version, description, success, checksum, execution_time)
         VALUES (999, 'unknown test migration', true, decode('00', 'hex'), 0)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        schema::check_compatibility(&pool).await,
        Err(SchemaCompatibilityError::Unknown(999))
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn dirty_and_checksum_mismatched_migrations_are_rejected(pool: PgPool) {
    let latest = sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET success = false WHERE version = $1")
        .bind(latest)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        schema::check_compatibility(&pool).await,
        Err(SchemaCompatibilityError::Dirty(version)) if version == latest
    ));

    sqlx::query(
        "UPDATE _sqlx_migrations
         SET success = true, checksum = decode('00', 'hex')
         WHERE version = $1",
    )
    .bind(latest)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        schema::check_compatibility(&pool).await,
        Err(SchemaCompatibilityError::ChecksumMismatch(version)) if version == latest
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn readiness_fails_closed_during_database_outage_while_liveness_stays_up(pool: PgPool) {
    pool.close().await;
    let app = api::router(AppState::new(pool));
    let health = app
        .clone()
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);

    let ready = app
        .oneshot(Request::get("/api/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(ready.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body(ready).await,
        json!({"error":{"code":"service_unavailable","message":"service is not ready"}})
    );
}
