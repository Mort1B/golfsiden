use super::*;
use crate::{
    domain::{
        leaderboards::LeaderboardMetric,
        result_sharing::{PublicResults, ResultShareToken, project},
    },
    repositories::leaderboards,
};
use sqlx::{FromRow, PgPool};
#[derive(FromRow)]
struct Grant {
    tournament_id: Uuid,
    token_hash: Vec<u8>,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}
pub struct PublicRead {
    pub grant_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub results: PublicResults,
}
pub async fn read(
    pool: &PgPool,
    id: Uuid,
    token: &ResultShareToken,
    metric: LeaderboardMetric,
) -> Result<PublicRead, ShareError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    // FOR SHARE serializes a delivered authorized snapshot before rotation or
    // revocation. A queued reader seeing an updated RR row is unavailable.
    let grant=sqlx::query_as::<_,Grant>("SELECT tournament_id,token_hash,expires_at,revoked_at FROM tournament_result_shares WHERE id=$1 FOR SHARE")
        .bind(id).fetch_optional(&mut *tx).await.map_err(|error| {
            if error.as_database_error().is_some_and(|e|e.code().as_deref()==Some("40001")) {ShareError::Unavailable} else {ShareError::Database(error)}
        })?.ok_or(ShareError::Unavailable)?;
    if !token.matches(&grant.token_hash) || grant.revoked_at.is_some() {
        return Err(ShareError::Unavailable);
    }
    check_expiry(&mut tx, grant.expires_at).await?;
    let name = sqlx::query_scalar("SELECT name FROM tournaments WHERE id=$1")
        .bind(grant.tournament_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ShareError::Unavailable)?;
    let board = leaderboards::public_in_transaction(&mut tx, grant.tournament_id, metric)
        .await
        .map_err(|e| match e {
            leaderboards::LeaderboardError::Database(error) => ShareError::Database(error),
            _ => ShareError::InvalidResults,
        })?;
    let results = project(name, board);
    check_expiry(&mut tx, grant.expires_at).await?;
    tx.commit().await?;
    Ok(PublicRead {
        grant_id: id,
        expires_at: grant.expires_at,
        results,
    })
}
async fn check_expiry(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    expires: DateTime<Utc>,
) -> Result<(), ShareError> {
    if !sqlx::query_scalar::<_, bool>("SELECT $1>clock_timestamp()")
        .bind(expires)
        .fetch_one(&mut **tx)
        .await?
    {
        return Err(ShareError::Unavailable);
    }
    Ok(())
}
