use super::{authorization::*, *};
use crate::domain::password_recovery::RecoveryTokenHash;
use sqlx::PgPool;

struct NewGrant<'a> {
    target: &'a Account,
    issuer: Option<Uuid>,
    tournament: Option<Uuid>,
    reason: &'a str,
}
async fn issue(
    tx: &mut Transaction<'_, Postgres>,
    request: NewGrant<'_>,
    hash: &RecoveryTokenHash,
) -> Result<GrantMetadata, RecoveryError> {
    finish(
        tx,
        request.target.id,
        "replaced",
        request.issuer,
        request.reason,
        None,
    )
    .await?;
    let version = watermark(tx).await?;
    Ok(sqlx::query_as("WITH instant AS (SELECT clock_timestamp() AS now) INSERT INTO password_recovery_grants(id,token_hash,target_user_id,target_player_id,credential_generation,authority_version,issuer_kind,issuer_user_id,tournament_id,reason,issued_at,expires_at) SELECT $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,now,now+interval '30 minutes' FROM instant RETURNING id,expires_at")
        .bind(Uuid::new_v4()).bind(hash.as_bytes()).bind(request.target.id).bind(request.target.player_id).bind(request.target.credential_generation).bind(version)
        .bind(if request.issuer.is_some(){"tournament_admin"}else{"operator"}).bind(request.issuer).bind(request.tournament).bind(request.reason).fetch_one(&mut **tx).await?)
}
pub async fn admin_issue(
    pool: &PgPool,
    request: &AdminRequest,
    hash: &RecoveryTokenHash,
) -> Result<GrantMetadata, RecoveryError> {
    let mut tx = pool.begin().await?;
    let target = admin_target(&mut tx, request).await?;
    let grant = issue(
        &mut tx,
        NewGrant {
            target: &target,
            issuer: Some(request.user_id),
            tournament: Some(request.tournament_id),
            reason: "Tournament administrator issued recovery link",
        },
        hash,
    )
    .await?;
    require_session(&mut tx, request).await?;
    tx.commit().await?;
    Ok(grant)
}
pub async fn admin_revoke(pool: &PgPool, request: &AdminRequest) -> Result<(), RecoveryError> {
    let mut tx = pool.begin().await?;
    let target = admin_target(&mut tx, request).await?;
    finish(
        &mut tx,
        target.id,
        "revoked",
        Some(request.user_id),
        "Organizer revoked recovery",
        Some(request.tournament_id),
    )
    .await?;
    require_session(&mut tx, request).await?;
    tx.commit().await?;
    Ok(())
}
async fn operator_target(
    tx: &mut Transaction<'_, Postgres>,
    target: Uuid,
    reason: &str,
) -> Result<Account, RecoveryError> {
    if reason.trim().is_empty() || reason.chars().count() > 500 {
        return Err(RecoveryError::Forbidden);
    }
    let allowed: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_class c JOIN pg_roles r ON r.oid=c.relowner WHERE c.oid='password_recovery_grants'::regclass AND r.rolname=current_user) OR EXISTS(SELECT 1 FROM pg_roles WHERE rolname=current_user AND rolsuper)")
        .fetch_one(&mut **tx).await?;
    if !allowed {
        return Err(RecoveryError::Forbidden);
    }
    accounts(tx, &[target])
        .await?
        .pop()
        .ok_or(RecoveryError::Forbidden)
}
pub async fn operator_issue(
    pool: &PgPool,
    target: Uuid,
    reason: &str,
    hash: &RecoveryTokenHash,
) -> Result<GrantMetadata, RecoveryError> {
    let mut tx = pool.begin().await?;
    let target = operator_target(&mut tx, target, reason).await?;
    let grant = issue(
        &mut tx,
        NewGrant {
            target: &target,
            issuer: None,
            tournament: None,
            reason,
        },
        hash,
    )
    .await?;
    tx.commit().await?;
    Ok(grant)
}
pub async fn operator_revoke(
    pool: &PgPool,
    target: Uuid,
    reason: &str,
) -> Result<(), RecoveryError> {
    let mut tx = pool.begin().await?;
    let target = operator_target(&mut tx, target, reason).await?;
    finish(&mut tx, target.id, "revoked", None, reason, None).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn operator_revoke_exact(
    pool: &PgPool,
    target: Uuid,
    id: Uuid,
    reason: &str,
) -> Result<(), RecoveryError> {
    let mut tx = pool.begin().await?;
    operator_target(&mut tx, target, reason).await?;
    sqlx::query("UPDATE password_recovery_grants SET outcome='revoked',ended_at=clock_timestamp(),end_reason=$3 WHERE target_user_id=$1 AND id=$2 AND outcome IS NULL")
        .bind(target).bind(id).bind(reason).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
