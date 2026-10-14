use std::future::Future;

use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::repositories::{auth, tournament_authorization::AuthorizationError};

#[derive(Debug, Error)]
pub enum SessionReadError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("private read temporarily unavailable")]
    Unavailable,
}

pub(crate) async fn begin(
    pool: &PgPool,
    session: Uuid,
) -> Result<(Transaction<'_, Postgres>, Uuid), SessionReadError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    // Hold session/user authority before memberships, through all result loading.
    let user = auth::lock_active_session(&mut tx, session)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?
        .user_id;
    Ok((tx, user))
}

pub(crate) async fn finish(
    mut tx: Transaction<'_, Postgres>,
    session: Uuid,
) -> Result<(), SessionReadError> {
    // Row locks serialize revocation and credential replacement; only a fresh
    // database wall-clock check can reject natural expiry during a later wait.
    auth::lock_active_session(&mut tx, session)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    tx.commit().await?;
    Ok(())
}

fn serialization_failure(error: &SessionReadError) -> bool {
    match error {
        SessionReadError::Database(error)
        | SessionReadError::Authorization(AuthorizationError::Database(error)) => error
            .as_database_error()
            .is_some_and(|error| error.code().as_deref() == Some("40001")),
        _ => false,
    }
}

pub(crate) async fn retry<T, F, Fut>(mut read: F) -> Result<T, SessionReadError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, SessionReadError>>,
{
    // Retry only complete reads: the failed snapshot's user, roles and payload
    // are discarded. A second conflict has the established generic 503 contract.
    match read().await {
        Err(error) if serialization_failure(&error) => {}
        result => return result,
    }
    match read().await {
        Err(error) if serialization_failure(&error) => Err(SessionReadError::Unavailable),
        result => result,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[cfg(feature = "database-tests")]
    #[sqlx::test(migrations = "../migrations")]
    async fn snapshot_conflicts_retry_once_for_direct_and_nested_errors(pool: PgPool) {
        for nested in [false, true] {
            let attempts = AtomicUsize::new(0);
            let result: Result<(), _> = retry(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                let error = sqlx::query("DO $$ BEGIN RAISE EXCEPTION 'test snapshot conflict' USING ERRCODE='40001'; END $$").execute(&pool).await.unwrap_err();
                Err(if nested { SessionReadError::Authorization(AuthorizationError::Database(error)) } else { SessionReadError::Database(error) })
            }).await;
            assert!(matches!(result, Err(SessionReadError::Unavailable)));
            assert_eq!(attempts.load(Ordering::SeqCst), 2);
        }
    }

    #[tokio::test]
    async fn nonserialization_errors_are_never_retried() {
        let attempts = AtomicUsize::new(0);
        let result: Result<(), _> = retry(|| async {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(SessionReadError::Authorization(
                AuthorizationError::Forbidden,
            ))
        })
        .await;
        assert!(matches!(
            result,
            Err(SessionReadError::Authorization(
                AuthorizationError::Forbidden
            ))
        ));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}
