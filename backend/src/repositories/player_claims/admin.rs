use super::{ClaimError, authorize, lock_player, recheck_session, require_unlinked};
use crate::repositories::tournament_authorization::AuthorizationError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Serialize, FromRow)]
pub struct Grant {
    pub player_id: Uuid,
    pub claim_id: Uuid,
    pub expires_at: DateTime<Utc>,
}
#[derive(Serialize, FromRow)]
pub struct AccountState {
    pub player_id: Uuid,
    pub has_account: bool,
    pub claim_id: Option<Uuid>,
    pub claim_expires_at: Option<DateTime<Utc>>,
    pub claim_revoked_at: Option<DateTime<Utc>>,
    pub claimed_at: Option<DateTime<Utc>>,
}
pub async fn list(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
) -> Result<Vec<AccountState>, ClaimError> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, session, tournament).await?;
    let rows=sqlx::query_as("SELECT tp.player_id, EXISTS(SELECT 1 FROM users u WHERE u.player_id=tp.player_id) AS has_account, g.id AS claim_id, g.expires_at AS claim_expires_at,g.revoked_at AS claim_revoked_at,g.claimed_at FROM tournament_players tp LEFT JOIN LATERAL (SELECT id,expires_at,revoked_at,claimed_at FROM player_claim_grants WHERE tournament_id=tp.tournament_id AND player_id=tp.player_id ORDER BY created_at DESC,id DESC LIMIT 1) g ON true WHERE tp.tournament_id=$1 ORDER BY tp.player_id")
        .bind(tournament).fetch_all(&mut *tx).await?;
    recheck_session(&mut tx, session).await?;
    tx.commit().await?;
    Ok(rows)
}
async fn parent(
    tx: &mut Transaction<'_, Postgres>,
    tournament: Uuid,
    closed_ok: bool,
) -> Result<(), ClaimError> {
    let closed: bool = sqlx::query_scalar(
        "SELECT status IN ('completed','archived') FROM tournaments WHERE id=$1 FOR SHARE",
    )
    .bind(tournament)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AuthorizationError::NotFound)?;
    if closed && !closed_ok {
        return Err(ClaimError::Closed);
    }
    Ok(())
}
async fn insert_grant(
    tx: &mut Transaction<'_, Postgres>,
    tournament: Uuid,
    player: Uuid,
    actor: Uuid,
    hash: &[u8],
) -> Result<Grant, ClaimError> {
    sqlx::query("UPDATE player_claim_grants SET revoked_at=clock_timestamp() WHERE player_id=$1 AND revoked_at IS NULL AND claimed_at IS NULL")
        .bind(player).execute(&mut **tx).await?;
    Ok(sqlx::query_as("INSERT INTO player_claim_grants(id,tournament_id,player_id,token_hash,created_by_user_id) VALUES($1,$2,$3,$4,$5) RETURNING player_id,id AS claim_id,expires_at")
        .bind(Uuid::new_v4()).bind(tournament).bind(player).bind(hash).bind(actor).fetch_one(&mut **tx).await?)
}
pub async fn create(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
    name: &str,
    handicap: f64,
    hash: &[u8],
) -> Result<Grant, ClaimError> {
    let mut tx = pool.begin().await?;
    let actor = authorize(&mut tx, session, tournament).await?;
    parent(&mut tx, tournament, false).await?;
    let player = Uuid::new_v4();
    sqlx::query("INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,$2,$3)")
        .bind(player)
        .bind(name)
        .bind(handicap)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES($1,$2,$3)").bind(tournament).bind(player).bind(handicap).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO handicap_history(id,player_id,handicap_index,changed_by,reason) VALUES($1,$2,$3,$4,'administrator prepared player')").bind(Uuid::new_v4()).bind(player).bind(handicap).bind(actor).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO tournament_handicap_history(id,tournament_id,player_id,handicap_index,changed_by,reason) VALUES($1,$2,$3,$4,$5,'administrator prepared entrant')").bind(Uuid::new_v4()).bind(tournament).bind(player).bind(handicap).bind(actor).execute(&mut *tx).await?;
    let grant = insert_grant(&mut tx, tournament, player, actor, hash).await?;
    recheck_session(&mut tx, session).await?;
    tx.commit().await?;
    Ok(grant)
}
pub async fn reissue(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
    player: Uuid,
    hash: &[u8],
) -> Result<Grant, ClaimError> {
    let mut tx = pool.begin().await?;
    let actor = authorize(&mut tx, session, tournament).await?;
    parent(&mut tx, tournament, true).await?;
    lock_player(&mut tx, tournament, player).await?;
    require_unlinked(&mut tx, player).await?;
    let grant = insert_grant(&mut tx, tournament, player, actor, hash).await?;
    recheck_session(&mut tx, session).await?;
    tx.commit().await?;
    Ok(grant)
}
pub async fn revoke(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
    player: Uuid,
) -> Result<(), ClaimError> {
    let mut tx = pool.begin().await?;
    authorize(&mut tx, session, tournament).await?;
    parent(&mut tx, tournament, true).await?;
    lock_player(&mut tx, tournament, player).await?;
    sqlx::query("UPDATE player_claim_grants SET revoked_at=clock_timestamp() WHERE tournament_id=$1 AND player_id=$2 AND claimed_at IS NULL AND revoked_at IS NULL")
        .bind(tournament).bind(player).execute(&mut *tx).await?;
    recheck_session(&mut tx, session).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn withdraw(
    pool: &PgPool,
    session: Uuid,
    tournament: Uuid,
    player: Uuid,
) -> Result<(), ClaimError> {
    let mut preflight = pool.begin().await?;
    authorize(&mut preflight, session, tournament).await?;
    preflight.rollback().await?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE")
        .bind(tournament)
        .fetch_all(&mut *tx)
        .await?;
    authorize(&mut tx, session, tournament).await?;
    // Excludes new rounds while preserving round-before-parent lock order.
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR UPDATE")
        .bind(tournament)
        .execute(&mut *tx)
        .await?;
    parent(&mut tx, tournament, false).await?;
    sqlx::query("SELECT id FROM users WHERE player_id=$1 FOR SHARE NOWAIT")
        .bind(player)
        .fetch_all(&mut *tx)
        .await?;
    sqlx::query("SELECT m.user_id FROM tournament_memberships m JOIN users u ON u.id=m.user_id WHERE u.player_id=$1 AND m.tournament_id=$2 FOR SHARE OF m NOWAIT")
        .bind(player).bind(tournament).fetch_all(&mut *tx).await?;
    lock_player(&mut tx, tournament, player).await?;
    sqlx::query("SELECT set_config('app.player_withdrawal_session',$1,true)")
        .bind(session.to_string())
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE tournament_players SET status='withdrawn' WHERE tournament_id=$1 AND player_id=$2",
    )
    .bind(tournament)
    .bind(player)
    .execute(&mut *tx)
    .await?;
    recheck_session(&mut tx, session).await?;
    tx.commit().await?;
    Ok(())
}
