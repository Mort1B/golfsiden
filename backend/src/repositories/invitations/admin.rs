use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    InvitationError, InvitationMetadata, classify_database, lock_admin, lock_series_and_count,
};

const METADATA_COLUMNS: &str = "i.id, i.tournament_id, i.series_id, i.predecessor_id,
     i.created_by_user_id, i.created_at, i.expires_at, i.revoked_at,
     i.revoked_by_user_id, i.revocation_actor_known, i.max_uses,
     (SELECT count(*) FROM invitation_redemptions r
      WHERE r.tournament_id = i.tournament_id
        AND r.series_id = i.series_id) AS redemption_count";

pub async fn list(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
) -> Result<Vec<InvitationMetadata>, InvitationError> {
    let mut transaction = pool.begin().await.map_err(InvitationError::Database)?;
    lock_admin(&mut transaction, session_id, tournament_id).await?;
    let rows = sqlx::query_as::<_, InvitationMetadata>(&format!(
        "SELECT {METADATA_COLUMNS}
         FROM tournament_invitations i
         WHERE i.tournament_id = $1
         ORDER BY i.created_at DESC, i.id DESC"
    ))
    .bind(tournament_id)
    .fetch_all(&mut *transaction)
    .await
    .map_err(InvitationError::Database)?;
    transaction.commit().await.map_err(classify_database)?;
    Ok(rows)
}

pub struct IssueParams<'a> {
    pub session_id: Uuid,
    pub tournament_id: Uuid,
    pub invitation_id: Uuid,
    pub token_hash: &'a [u8],
    pub expires_at: DateTime<Utc>,
    pub max_uses: Option<i32>,
}

pub async fn issue(
    pool: &PgPool,
    params: IssueParams<'_>,
) -> Result<InvitationMetadata, InvitationError> {
    let mut transaction = pool.begin().await.map_err(InvitationError::Database)?;
    let actor = lock_admin(&mut transaction, params.session_id, params.tournament_id).await?;
    let metadata = sqlx::query_as::<_, InvitationMetadata>(
        "WITH inserted AS (
           INSERT INTO tournament_invitations
             (id, tournament_id, token_hash, created_by_user_id, expires_at,
              max_uses, series_id)
           VALUES ($1, $2, $3, $4, $5, $6, $1)
           RETURNING *
         )
         SELECT i.id, i.tournament_id, i.series_id, i.predecessor_id,
                i.created_by_user_id, i.created_at, i.expires_at, i.revoked_at,
                i.revoked_by_user_id, i.revocation_actor_known, i.max_uses,
                (SELECT count(*) FROM invitation_redemptions r
                 WHERE r.tournament_id = i.tournament_id
                   AND r.series_id = i.series_id) AS redemption_count
         FROM inserted i",
    )
    .bind(params.invitation_id)
    .bind(params.tournament_id)
    .bind(params.token_hash)
    .bind(actor)
    .bind(params.expires_at)
    .bind(params.max_uses)
    .fetch_one(&mut *transaction)
    .await
    .map_err(classify_database)?;
    transaction.commit().await.map_err(classify_database)?;
    Ok(metadata)
}

pub struct RotateParams<'a> {
    pub session_id: Uuid,
    pub tournament_id: Uuid,
    pub predecessor_id: Uuid,
    pub successor_id: Uuid,
    pub token_hash: &'a [u8],
}

pub async fn rotate(
    pool: &PgPool,
    params: RotateParams<'_>,
) -> Result<InvitationMetadata, InvitationError> {
    let mut transaction = pool.begin().await.map_err(InvitationError::Database)?;
    let actor = lock_admin(&mut transaction, params.session_id, params.tournament_id).await?;
    let predecessor = sqlx::query_as::<_, AdminInvitation>(
        "SELECT id, tournament_id, series_id, expires_at, revoked_at, max_uses
         FROM tournament_invitations
         WHERE tournament_id = $1 AND id = $2
         FOR UPDATE",
    )
    .bind(params.tournament_id)
    .bind(params.predecessor_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(InvitationError::Database)?
    .ok_or(InvitationError::NotFound)?;
    let locked = predecessor.as_locked();
    let redemption_count = lock_series_and_count(&mut transaction, &locked)
        .await
        .map_err(InvitationError::Database)?;
    let checked_at = super::transaction_clock(&mut transaction)
        .await
        .map_err(InvitationError::Database)?;
    if predecessor.revoked_at.is_some()
        || predecessor.expires_at <= checked_at
        || matches!(predecessor.max_uses, Some(maximum) if redemption_count >= i64::from(maximum))
    {
        return Err(InvitationError::RotationConflict);
    }
    sqlx::query(
        "UPDATE tournament_invitations
         SET revoked_at = $3, revoked_by_user_id = $4
         WHERE tournament_id = $1 AND id = $2",
    )
    .bind(params.tournament_id)
    .bind(params.predecessor_id)
    .bind(checked_at)
    .bind(actor)
    .execute(&mut *transaction)
    .await
    .map_err(classify_database)?;
    let metadata = sqlx::query_as::<_, InvitationMetadata>(
        "WITH inserted AS (
           INSERT INTO tournament_invitations
             (id, tournament_id, token_hash, created_by_user_id, expires_at,
              max_uses, series_id, predecessor_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING *
         )
         SELECT i.id, i.tournament_id, i.series_id, i.predecessor_id,
                i.created_by_user_id, i.created_at, i.expires_at, i.revoked_at,
                i.revoked_by_user_id, i.revocation_actor_known, i.max_uses,
                (SELECT count(*) FROM invitation_redemptions r
                 WHERE r.tournament_id = i.tournament_id
                   AND r.series_id = i.series_id) AS redemption_count
         FROM inserted i",
    )
    .bind(params.successor_id)
    .bind(params.tournament_id)
    .bind(params.token_hash)
    .bind(actor)
    .bind(predecessor.expires_at)
    .bind(predecessor.max_uses)
    .bind(predecessor.series_id)
    .bind(predecessor.id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(classify_database)?;
    transaction.commit().await.map_err(classify_database)?;
    Ok(metadata)
}

pub async fn revoke(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    invitation_id: Uuid,
) -> Result<(), InvitationError> {
    let mut transaction = pool.begin().await.map_err(InvitationError::Database)?;
    let actor = lock_admin(&mut transaction, session_id, tournament_id).await?;
    let revoked_at = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT revoked_at FROM tournament_invitations
         WHERE tournament_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(tournament_id)
    .bind(invitation_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(InvitationError::Database)?
    .ok_or(InvitationError::NotFound)?;
    if revoked_at.is_none() {
        sqlx::query(
            "UPDATE tournament_invitations
             SET revoked_at = clock_timestamp(), revoked_by_user_id = $3
             WHERE tournament_id = $1 AND id = $2",
        )
        .bind(tournament_id)
        .bind(invitation_id)
        .bind(actor)
        .execute(&mut *transaction)
        .await
        .map_err(classify_database)?;
    }
    transaction.commit().await.map_err(classify_database)?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
struct AdminInvitation {
    id: Uuid,
    tournament_id: Uuid,
    series_id: Uuid,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    max_uses: Option<i32>,
}

impl AdminInvitation {
    fn as_locked(&self) -> super::LockedInvitation {
        super::LockedInvitation {
            id: self.id,
            tournament_id: self.tournament_id,
            series_id: self.series_id,
            token_hash: Vec::new(),
            expires_at: self.expires_at,
            revoked_at: self.revoked_at,
            max_uses: self.max_uses,
            tournament_status: crate::domain::models::TournamentStatus::Draft,
        }
    }
}
