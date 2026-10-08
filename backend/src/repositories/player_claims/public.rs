use super::{ClaimError, lock_player, require_unlinked};
use crate::{
    auth::verify_invitation_token_hash,
    repositories::{auth, invitations::RegisteredPlayer},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(FromRow)]
struct Identity {
    tournament_id: Uuid,
    player_id: Uuid,
    token_hash: Vec<u8>,
}
#[derive(Serialize)]
pub struct ClaimPreview {
    pub tournament: PreviewTournament,
    pub player: PreviewPlayer,
    pub expires_at: DateTime<Utc>,
}
#[derive(Serialize)]
pub struct PreviewTournament {
    pub id: Uuid,
    pub name: String,
}
#[derive(Serialize)]
pub struct PreviewPlayer {
    pub id: Uuid,
    pub display_name: String,
}
async fn identity(pool: &PgPool, id: Uuid, token: &str) -> Result<Identity, ClaimError> {
    let row = sqlx::query_as::<_, Identity>(
        "SELECT tournament_id,player_id,token_hash FROM player_claim_grants WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    if !verify_invitation_token_hash(
        token,
        row.as_ref()
            .map(|r| r.token_hash.as_slice())
            .unwrap_or(&[0; 32]),
    ) {
        return Err(ClaimError::Invalid);
    }
    row.ok_or(ClaimError::Invalid)
}
async fn lock_target(
    tx: &mut Transaction<'_, Postgres>,
    identity: &Identity,
) -> Result<(), ClaimError> {
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR SHARE")
        .bind(identity.tournament_id)
        .execute(&mut **tx)
        .await?;
    lock_player(tx, identity.tournament_id, identity.player_id)
        .await
        .map_err(|e| match e {
            ClaimError::Withdrawn => ClaimError::Unavailable,
            other => other,
        })?;
    require_unlinked(tx, identity.player_id)
        .await
        .map_err(|e| match e {
            ClaimError::HasAccount => ClaimError::Unavailable,
            other => other,
        })?;
    Ok(())
}
async fn check_grant(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    token: &str,
) -> Result<DateTime<Utc>, ClaimError> {
    let (hash,expires,revoked,claimed)=sqlx::query_as::<_,(Vec<u8>,DateTime<Utc>,Option<DateTime<Utc>>,Option<DateTime<Utc>>)>("SELECT token_hash,expires_at,revoked_at,claimed_at FROM player_claim_grants WHERE id=$1 FOR UPDATE")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(ClaimError::Invalid)?;
    if !verify_invitation_token_hash(token, &hash) {
        return Err(ClaimError::Invalid);
    }
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    if expires <= now || revoked.is_some() || claimed.is_some() {
        return Err(ClaimError::Unavailable);
    }
    Ok(expires)
}
pub async fn preview(pool: &PgPool, id: Uuid, token: &str) -> Result<ClaimPreview, ClaimError> {
    let identity = identity(pool, id, token).await?;
    let mut tx = pool.begin().await?;
    lock_target(&mut tx, &identity).await?;
    let expires_at = check_grant(&mut tx, id, token).await?;
    let name = sqlx::query_scalar("SELECT name FROM tournaments WHERE id=$1")
        .bind(identity.tournament_id)
        .fetch_one(&mut *tx)
        .await?;
    let display_name = sqlx::query_scalar("SELECT display_name FROM players WHERE id=$1")
        .bind(identity.player_id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ClaimPreview {
        tournament: PreviewTournament {
            id: identity.tournament_id,
            name,
        },
        player: PreviewPlayer {
            id: identity.player_id,
            display_name,
        },
        expires_at,
    })
}
pub struct RegisterParams<'a> {
    pub id: Uuid,
    pub token: &'a str,
    pub username: &'a str,
    pub password_hash: &'a str,
    pub session_token_hash: &'a [u8],
    pub session_expires_at: DateTime<Utc>,
}
pub async fn register(
    pool: &PgPool,
    p: RegisterParams<'_>,
) -> Result<RegisteredPlayer, ClaimError> {
    let identity = identity(pool, p.id, p.token).await?;
    let mut tx = pool.begin().await?;
    lock_target(&mut tx, &identity).await?;
    check_grant(&mut tx, p.id, p.token).await?;
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,role,password_hash,player_id) SELECT $1,$2,display_name,'player',$3,id FROM players WHERE id=$4")
        .bind(user).bind(p.username).bind(p.password_hash).bind(identity.player_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE player_claim_grants SET claimed_at=clock_timestamp(),claimed_by_user_id=$2,claimed_transaction=pg_current_xact_id() WHERE id=$1")
        .bind(p.id).bind(user).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'player')",
    )
    .bind(identity.tournament_id)
    .bind(user)
    .execute(&mut *tx)
    .await?;
    let session = auth::create_session_in_transaction(
        &mut tx,
        user,
        p.session_token_hash,
        p.session_expires_at,
    )
    .await?;
    // Username uniqueness and trigger waits can outlive the original check.
    let valid:bool=sqlx::query_scalar("SELECT expires_at>clock_timestamp() AND revoked_at IS NULL FROM player_claim_grants WHERE id=$1").bind(p.id).fetch_one(&mut *tx).await?;
    if !valid {
        return Err(ClaimError::Unavailable);
    }
    tx.commit().await?;
    Ok(RegisteredPlayer {
        session,
        tournament_id: identity.tournament_id,
        player_id: identity.player_id,
    })
}
