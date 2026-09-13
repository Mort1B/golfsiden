mod authorization;
mod mutations;
mod public;

use chrono::{DateTime, Utc};
pub use mutations::{
    admin_issue, admin_revoke, operator_issue, operator_revoke, operator_revoke_exact,
};
pub use public::{preview, redeem};
use sqlx::{FromRow, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("recovery link is invalid or no longer available")]
    Invalid,
    #[error("request is not permitted")]
    Forbidden,
    #[error("authentication required")]
    Unauthenticated,
    #[error("current password is incorrect")]
    IncorrectPassword,
    #[error("recovery database operation failed")]
    Database(#[from] sqlx::Error),
}
#[derive(FromRow)]
pub struct GrantMetadata {
    pub id: Uuid,
    pub expires_at: DateTime<Utc>,
}
#[derive(FromRow)]
struct Grant {
    id: Uuid,
    token_hash: Vec<u8>,
    target_user_id: Uuid,
    target_player_id: Option<Uuid>,
    credential_generation: i64,
    authority_version: i64,
    issuer_kind: String,
    issuer_user_id: Option<Uuid>,
    tournament_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
    outcome: Option<String>,
}
/// Verified before entering the transaction, rechecked after all serialization waits.
pub struct AdminRequest {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub tournament_id: Uuid,
    pub player_id: Uuid,
    pub verified_password: (String, i64),
}
#[derive(FromRow)]
struct Account {
    id: Uuid,
    player_id: Option<Uuid>,
    password_hash: Option<String>,
    credential_generation: i64,
    is_admin: bool,
}
async fn accounts(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<Vec<Account>, RecoveryError> {
    Ok(sqlx::query_as("SELECT id,player_id,password_hash,credential_generation,role='admin' AS is_admin FROM users WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(ids).fetch_all(&mut **tx).await?)
}
async fn watermark(tx: &mut Transaction<'_, Postgres>) -> Result<i64, RecoveryError> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(max(version),0) FROM password_recovery_authority_changes",
    )
    .fetch_one(&mut **tx)
    .await?)
}
async fn finish(
    tx: &mut Transaction<'_, Postgres>,
    target: Uuid,
    outcome: &str,
    actor: Option<Uuid>,
    reason: &str,
    context: Option<Uuid>,
) -> Result<(), RecoveryError> {
    sqlx::query("UPDATE password_recovery_grants SET outcome=$2,ended_at=clock_timestamp(),ended_by=$3,end_reason=$4 WHERE target_user_id=$1 AND outcome IS NULL AND ($5::uuid IS NULL OR tournament_id=$5)")
        .bind(target).bind(outcome).bind(actor).bind(reason).bind(context).execute(&mut **tx).await?;
    Ok(())
}
