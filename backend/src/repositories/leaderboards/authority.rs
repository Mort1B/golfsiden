use std::future::Future;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{AuthorizationError, LeaderboardError};
use crate::repositories::auth;

pub(super) async fn begin(
    pool: &PgPool,
    session: Option<Uuid>,
) -> Result<(Transaction<'_, Postgres>, Option<Uuid>), LeaderboardError> {
    let mut tx = pool.begin().await?;
    sqlx::query(if session.is_some() {
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ"
    } else {
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY"
    })
    .execute(&mut *tx)
    .await?;
    // Session/user locks precede membership locks. Revocation and credential
    // changes must serialize with the same authority used to assemble results.
    let user = if let Some(session) = session {
        Some(
            auth::lock_active_session(&mut tx, session)
                .await?
                .ok_or(AuthorizationError::Unauthenticated)?
                .user_id,
        )
    } else {
        None
    };
    Ok((tx, user))
}

pub(super) async fn finish(
    mut tx: Transaction<'_, Postgres>,
    session: Option<Uuid>,
) -> Result<(), LeaderboardError> {
    // Locks protect revocation/generation, but wall-clock expiry can advance
    // during any membership or fact-loading wait, including match-only reads.
    if let Some(session) = session {
        auth::lock_active_session(&mut tx, session)
            .await?
            .ok_or(AuthorizationError::Unauthenticated)?;
    }
    tx.commit().await?;
    Ok(())
}

fn serialization_failure(error: &LeaderboardError) -> bool {
    match error {
        LeaderboardError::Database(error)
        | LeaderboardError::Authorization(AuthorizationError::Database(error)) => error
            .as_database_error()
            .is_some_and(|error| error.code().as_deref() == Some("40001")),
        _ => false,
    }
}

pub(super) async fn retry<T, F, Fut>(mut read: F) -> Result<T, LeaderboardError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, LeaderboardError>>,
{
    // Discard the entire old snapshot; never reuse its user, role or facts.
    // Only read transactions retry, with one fresh attempt before a generic 503.
    match read().await {
        Err(error) if serialization_failure(&error) => {}
        result => return result,
    }
    match read().await {
        Err(error) if serialization_failure(&error) => Err(LeaderboardError::ReadUnavailable),
        result => result,
    }
}

#[cfg(all(test, feature = "database-tests"))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[sqlx::test(migrations = "../migrations")]
    async fn repeated_snapshot_failures_are_bounded_for_both_error_paths(pool: PgPool) {
        for nested in [false, true] {
            let attempts = AtomicUsize::new(0);
            let result: Result<(), _> = retry(|| async {
                attempts.fetch_add(1, Ordering::SeqCst);
                let error = sqlx::query(
                    "DO $$ BEGIN RAISE EXCEPTION 'test snapshot conflict' USING ERRCODE='40001'; END $$",
                )
                .execute(&pool)
                .await
                .unwrap_err();
                Err(if nested {
                    LeaderboardError::Authorization(AuthorizationError::Database(error))
                } else {
                    LeaderboardError::Database(error)
                })
            })
            .await;
            assert!(matches!(result, Err(LeaderboardError::ReadUnavailable)));
            assert_eq!(attempts.load(Ordering::SeqCst), 2);
        }
    }
}
