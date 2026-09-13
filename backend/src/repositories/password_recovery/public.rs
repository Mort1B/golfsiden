use super::{authorization::*, *};
use crate::domain::password_recovery::RecoveryToken;
use sqlx::PgPool;

async fn lock_grant(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    token: &RecoveryToken,
) -> Result<Grant, RecoveryError> {
    // Read identity only to select deterministic account locks; reread the entire
    // immutable grant after waits before making any authority decision.
    let identity: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT target_user_id,issuer_user_id FROM password_recovery_grants WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    let (target, issuer) = identity.ok_or(RecoveryError::Invalid)?;
    let ids: Vec<_> = std::iter::once(target).chain(issuer).collect();
    let locked = accounts(tx, &ids).await?;
    let target = locked
        .iter()
        .find(|a| a.id == target)
        .ok_or(RecoveryError::Invalid)?;
    let grant: Grant=sqlx::query_as("SELECT id,token_hash,target_user_id,target_player_id,credential_generation,authority_version,issuer_kind,issuer_user_id,tournament_id,expires_at,outcome FROM password_recovery_grants WHERE id=$1 FOR UPDATE")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(RecoveryError::Invalid)?;
    if !token.matches(&grant.token_hash)
        || grant.outcome.is_some()
        || grant.credential_generation != target.credential_generation
    {
        return Err(RecoveryError::Invalid);
    }
    if grant.issuer_kind == "tournament_admin" {
        let issuer = grant.issuer_user_id.ok_or(RecoveryError::Invalid)?;
        if !locked.iter().any(|a| a.id == issuer) {
            return Err(RecoveryError::Invalid);
        }
        eligible(
            tx,
            issuer,
            target,
            grant.tournament_id.ok_or(RecoveryError::Invalid)?,
            grant.target_player_id.ok_or(RecoveryError::Invalid)?,
        )
        .await
        .map_err(|e| match e {
            RecoveryError::Database(_) => e,
            _ => RecoveryError::Invalid,
        })?;
        if !unchanged(tx, &grant).await? {
            return Err(RecoveryError::Invalid);
        }
    }
    require_unexpired(tx, id).await?;
    Ok(grant)
}
async fn require_unexpired(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(), RecoveryError> {
    let valid: bool = sqlx::query_scalar(
        "SELECT expires_at>clock_timestamp() FROM password_recovery_grants WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    if !valid {
        return Err(RecoveryError::Invalid);
    }
    Ok(())
}
pub async fn preview(
    pool: &PgPool,
    id: Uuid,
    token: &RecoveryToken,
) -> Result<GrantMetadata, RecoveryError> {
    let mut tx = pool.begin().await?;
    let grant = lock_grant(&mut tx, id, token).await?;
    tx.commit().await?;
    Ok(GrantMetadata {
        id: grant.id,
        expires_at: grant.expires_at,
    })
}
pub async fn redeem(
    pool: &PgPool,
    id: Uuid,
    token: &RecoveryToken,
    password_hash: &str,
) -> Result<(), RecoveryError> {
    let mut tx = pool.begin().await?;
    let grant = lock_grant(&mut tx, id, token).await?;
    sqlx::query("UPDATE users SET password_hash=$2 WHERE id=$1")
        .bind(grant.target_user_id)
        .bind(password_hash)
        .execute(&mut *tx)
        .await?;
    finish(
        &mut tx,
        grant.target_user_id,
        "redeemed",
        None,
        "Recovery link redeemed",
        None,
    )
    .await?;
    require_unexpired(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}
