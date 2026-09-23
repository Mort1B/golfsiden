use super::*;
use crate::{
    domain::result_sharing::ResultShareTokenHash,
    repositories::{auth, tournament_authorization},
};
use sqlx::{PgPool, Postgres, Transaction};

// Session/user and membership locks are already held, but natural expiry can
// occur during subsequent grant/audit writes. Failure rolls back the transaction.
async fn require_active_session(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
) -> Result<(), ShareError> {
    auth::lock_active_session(tx, session)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    Ok(())
}

async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    tournament: Uuid,
) -> Result<Uuid, ShareError> {
    Ok(tournament_authorization::require_tournament_admin(tx, session, tournament).await?)
}
async fn latest(
    tx: &mut Transaction<'_, Postgres>,
    tournament: Uuid,
    lock: bool,
) -> Result<Option<GrantMetadata>, ShareError> {
    let suffix = if lock { " FOR UPDATE" } else { "" };
    Ok(sqlx::query_as(&format!("SELECT {METADATA} FROM tournament_result_shares WHERE tournament_id=$1 ORDER BY created_at DESC,id DESC LIMIT 1{suffix}"))
        .bind(tournament).fetch_optional(&mut **tx).await?)
}
async fn management_lock(
    tx: &mut Transaction<'_, Postgres>,
    session: Uuid,
    tournament: Uuid,
) -> Result<(), ShareError> {
    authorize(tx, session, tournament).await?;
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR UPDATE")
        .bind(tournament)
        .execute(&mut **tx)
        .await?;
    sqlx::query("SELECT set_config('app.result_share_session_id',$1::text,true)")
        .bind(session)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub async fn status(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
) -> Result<Option<GrantMetadata>, ShareError> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, session, tournament).await?;
    let result = latest(&mut tx, tournament, false).await?;
    authorize(&mut tx, session, tournament).await?;
    tx.commit().await?;
    Ok(result)
}
pub async fn issue(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
    expected: Option<Uuid>,
    hash: &ResultShareTokenHash,
) -> Result<GrantMetadata, ShareError> {
    let mut tx = pool.begin().await?;
    management_lock(&mut tx, session, tournament).await?;
    let eligible = sqlx::query_scalar::<_, bool>(
        "SELECT counted_rounds IS NOT NULL FROM tournaments WHERE id=$1",
    )
    .bind(tournament)
    .fetch_one(&mut *tx)
    .await?;
    if !eligible {
        return Err(ShareError::OverallUnavailable);
    }
    let previous = latest(&mut tx, tournament, true).await?;
    let actor = authorize(&mut tx, session, tournament).await?;
    if previous.as_ref().map(|p| p.id) != expected {
        return Err(ShareError::Stale);
    }
    if let Some(previous) = previous.as_ref().filter(|p| p.revoked_at.is_none()) {
        sqlx::query("SELECT set_config('app.result_share_replacing','true',true)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE tournament_result_shares SET revoked_at=clock_timestamp(),revoked_by=$2 WHERE id=$1")
            .bind(previous.id).bind(actor).execute(&mut *tx).await?;
        // Detect expiry at the old grant's audit before the new grant's guard.
        require_active_session(&mut tx, session).await?;
    }
    let grant=sqlx::query_as(&format!("WITH stamp AS (SELECT GREATEST(clock_timestamp(),$5::timestamptz + interval '1 microsecond') AS created) INSERT INTO tournament_result_shares(id,tournament_id,token_hash,created_by,created_at,expires_at) SELECT $1,$2,$3,$4,created,created+interval '30 days' FROM stamp RETURNING {METADATA}"))
        .bind(Uuid::new_v4()).bind(tournament).bind(hash.as_bytes()).bind(actor).bind(previous.as_ref().map(|p|p.created_at))
        .fetch_one(&mut *tx).await?;
    require_active_session(&mut tx, session).await?;
    tx.commit().await?;
    Ok(grant)
}
pub async fn revoke(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
    grant: Uuid,
) -> Result<bool, ShareError> {
    let mut tx = pool.begin().await?;
    management_lock(&mut tx, session, tournament).await?;
    let previous = latest(&mut tx, tournament, true).await?;
    let actor = authorize(&mut tx, session, tournament).await?;
    let previous = previous
        .filter(|p| p.id == grant)
        .ok_or(ShareError::Stale)?;
    let changed = previous.revoked_at.is_none();
    if changed {
        sqlx::query("UPDATE tournament_result_shares SET revoked_at=clock_timestamp(),revoked_by=$2 WHERE id=$1")
            .bind(grant).bind(actor).execute(&mut *tx).await?;
    }
    require_active_session(&mut tx, session).await?;
    tx.commit().await?;
    Ok(changed)
}
