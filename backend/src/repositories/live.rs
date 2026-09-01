use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::{auth, tournament_authorization::AuthorizationError};

pub async fn authorize(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
) -> Result<(), AuthorizationError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tournaments WHERE id = $1)")
            .bind(tournament_id)
            .fetch_one(&mut *transaction)
            .await?;
    if !exists {
        return Err(AuthorizationError::NotFound);
    }
    let principal = auth::lock_active_session(&mut transaction, session_id)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    let membership = sqlx::query_scalar::<_, bool>(
        "SELECT true FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2
         FOR SHARE",
    )
    .bind(tournament_id)
    .bind(principal.user_id)
    .fetch_optional(&mut *transaction)
    .await?;
    if membership.is_none() {
        return Err(AuthorizationError::Forbidden);
    }
    transaction.commit().await?;
    Ok(())
}
