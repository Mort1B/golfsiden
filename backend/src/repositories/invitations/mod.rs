mod acceptance;
pub(crate) mod admin;
pub(crate) mod public;
mod registration;
mod writes;

pub use acceptance::accept;
pub use admin::{issue, list, revoke, rotate};
pub use public::{authenticate_token, preview};
pub use registration::{RegisterParams, register};

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{FromRow, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    auth::{SessionPrincipal, verify_invitation_token_hash},
    domain::{
        invitations::tournament_accepts_new_players,
        models::{TournamentRole, TournamentStatus},
    },
};

const DUMMY_INVITATION_HASH: [u8; 32] = [0_u8; 32];

#[derive(Debug, Error)]
pub enum InvitationError {
    #[error("invitation is invalid")]
    Invalid,
    #[error("invitation has expired")]
    Expired,
    #[error("invitation has been revoked")]
    Revoked,
    #[error("invitation has no remaining uses")]
    Exhausted,
    #[error("tournament is not accepting players")]
    TournamentNotJoinable,
    #[error("email is already registered")]
    DuplicateEmail,
    #[error("authentication required")]
    Unauthenticated,
    #[error("request is not permitted")]
    Forbidden,
    #[error("the account is not linked to a player")]
    UnlinkedPlayer,
    #[error("the linked player is inactive")]
    InactivePlayer,
    #[error("the tournament entrant is withdrawn")]
    WithdrawnPlayer,
    #[error("invitation cannot be rotated")]
    RotationConflict,
    #[error("resource not found")]
    NotFound,
    #[error("database operation failed")]
    Database(#[source] sqlx::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct InvitationPreview {
    pub tournament: PreviewTournament,
    pub invitation: PreviewInvitation,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewTournament {
    pub id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewInvitation {
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct InvitationMetadata {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub series_id: Uuid,
    pub predecessor_id: Option<Uuid>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by_user_id: Option<Uuid>,
    pub revocation_actor_known: bool,
    pub max_uses: Option<i32>,
    pub redemption_count: i64,
}

#[derive(Debug)]
pub struct RegisteredPlayer {
    pub session: SessionPrincipal,
    pub tournament_id: Uuid,
    pub player_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinStatus {
    Joined,
    AlreadyJoined,
}

#[derive(Debug, Serialize)]
pub struct AcceptedPlayer {
    pub status: JoinStatus,
    pub tournament_id: Uuid,
    pub player_id: Uuid,
}

#[derive(Debug, FromRow)]
struct LockedInvitation {
    id: Uuid,
    tournament_id: Uuid,
    series_id: Uuid,
    token_hash: Vec<u8>,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    max_uses: Option<i32>,
    tournament_status: TournamentStatus,
}

fn authenticate<T>(
    row: Option<T>,
    token: &str,
    stored_hash: impl Fn(&T) -> &[u8],
) -> Result<T, InvitationError> {
    let hash = row
        .as_ref()
        .map(&stored_hash)
        .unwrap_or(&DUMMY_INVITATION_HASH);
    let authenticated = verify_invitation_token_hash(token, hash);
    if !authenticated {
        return Err(InvitationError::Invalid);
    }
    row.ok_or(InvitationError::Invalid)
}

fn check_lifecycle(
    invitation: &LockedInvitation,
    redemption_count: i64,
    now: DateTime<Utc>,
) -> Result<(), InvitationError> {
    if invitation.revoked_at.is_some() {
        return Err(InvitationError::Revoked);
    }
    if invitation.expires_at <= now {
        return Err(InvitationError::Expired);
    }
    if !tournament_accepts_new_players(invitation.tournament_status) {
        return Err(InvitationError::TournamentNotJoinable);
    }
    if matches!(invitation.max_uses, Some(maximum) if redemption_count >= i64::from(maximum)) {
        return Err(InvitationError::Exhausted);
    }
    Ok(())
}

async fn lock_invitation(
    transaction: &mut Transaction<'_, Postgres>,
    invitation_id: Uuid,
) -> Result<Option<LockedInvitation>, sqlx::Error> {
    sqlx::query_as(
        "SELECT i.id, i.tournament_id, i.series_id, i.token_hash,
                i.expires_at, i.revoked_at, i.max_uses,
                t.status AS tournament_status
         FROM tournament_invitations i
         JOIN tournaments t ON t.id = i.tournament_id
         WHERE i.id = $1
         FOR UPDATE OF i",
    )
    .bind(invitation_id)
    .fetch_optional(&mut **transaction)
    .await
}

async fn lock_series_and_count(
    transaction: &mut Transaction<'_, Postgres>,
    invitation: &LockedInvitation,
) -> Result<i64, sqlx::Error> {
    if invitation.series_id != invitation.id {
        sqlx::query("SELECT id FROM tournament_invitations WHERE id = $1 FOR UPDATE")
            .bind(invitation.series_id)
            .execute(&mut **transaction)
            .await?;
    }
    sqlx::query_scalar(
        "SELECT count(*) FROM invitation_redemptions
         WHERE tournament_id = $1 AND series_id = $2",
    )
    .bind(invitation.tournament_id)
    .bind(invitation.series_id)
    .fetch_one(&mut **transaction)
    .await
}

async fn transaction_clock(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<DateTime<Utc>, sqlx::Error> {
    sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **transaction)
        .await
}

async fn lock_admin(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    tournament_id: Uuid,
) -> Result<Uuid, InvitationError> {
    let principal =
        crate::repositories::auth::lock_active_session_exclusive(transaction, session_id)
            .await
            .map_err(InvitationError::Database)?
            .ok_or(InvitationError::Unauthenticated)?;
    let tournament_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tournaments WHERE id = $1)")
            .bind(tournament_id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(InvitationError::Database)?;
    if !tournament_exists {
        return Err(InvitationError::NotFound);
    }
    let role = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(tournament_id)
    .bind(principal.user_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(InvitationError::Database)?;
    if role != Some(TournamentRole::Admin) {
        return Err(InvitationError::Forbidden);
    }
    Ok(principal.user_id)
}

fn classify_database(error: sqlx::Error) -> InvitationError {
    if let sqlx::Error::Database(database) = &error {
        match database.constraint() {
            Some("invitation_redemption_expired") => return InvitationError::Expired,
            Some("invitation_redemption_revoked") => return InvitationError::Revoked,
            Some("invitation_redemption_tournament_closed") => {
                return InvitationError::TournamentNotJoinable;
            }
            Some("invitation_redemption_capacity_exhausted") => {
                return InvitationError::Exhausted;
            }
            _ => {}
        }
        if database.is_unique_violation()
            && matches!(
                database.constraint(),
                Some("users_email_normalized_idx" | "users_email_key")
            )
        {
            return InvitationError::DuplicateEmail;
        }
        if database.is_unique_violation()
            && database.constraint() == Some("tournament_invitations_one_successor_idx")
        {
            return InvitationError::RotationConflict;
        }
    }
    InvitationError::Database(error)
}
