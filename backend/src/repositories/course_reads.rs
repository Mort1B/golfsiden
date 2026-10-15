use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{
    session_read::{self, SessionReadError},
    tournament_authorization::{self, AuthorizationError},
};

pub(super) async fn begin_admin(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
) -> Result<Transaction<'_, Postgres>, SessionReadError> {
    let (mut tx, user) = session_read::begin(pool, session).await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tournaments WHERE id=$1)")
        .bind(tournament)
        .fetch_one(&mut *tx)
        .await?;
    if !exists {
        return Err(AuthorizationError::NotFound.into());
    }
    tournament_authorization::require_tournament_admin_read_in_transaction(
        &mut tx, user, tournament,
    )
    .await?;
    Ok(tx)
}

// Only synchronous local projection belongs here. Provider/cache futures run
// between separately committed authority checks, never inside this transaction.
pub(crate) async fn local_for_session<T, F>(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
    project: F,
) -> Result<T, SessionReadError>
where
    F: Fn() -> T,
{
    session_read::retry(|| async {
        let tx = begin_admin(pool, session, tournament).await?;
        let result = project();
        session_read::finish(tx, session).await?;
        Ok(result)
    })
    .await
}
