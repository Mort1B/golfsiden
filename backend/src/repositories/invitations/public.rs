use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::models::TournamentStatus;

use super::{
    InvitationError, InvitationPreview, LockedInvitation, PreviewInvitation, PreviewTournament,
    authenticate, check_lifecycle,
};

#[derive(Debug, FromRow)]
struct PreviewRow {
    id: Uuid,
    tournament_id: Uuid,
    series_id: Uuid,
    token_hash: Vec<u8>,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    max_uses: Option<i32>,
    tournament_status: TournamentStatus,
    tournament_name: String,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
    redemption_count: i64,
}

pub async fn preview(
    pool: &PgPool,
    invitation_id: Uuid,
    token: &str,
    now: DateTime<Utc>,
) -> Result<InvitationPreview, InvitationError> {
    let row = sqlx::query_as::<_, PreviewRow>(
        "SELECT i.id, i.tournament_id, i.series_id, i.token_hash,
                i.expires_at, i.revoked_at, i.max_uses,
                t.status AS tournament_status, t.name AS tournament_name,
                t.start_date, t.end_date,
                (SELECT count(*) FROM invitation_redemptions r
                 WHERE r.tournament_id = i.tournament_id
                   AND r.series_id = i.series_id) AS redemption_count
         FROM tournament_invitations i
         JOIN tournaments t ON t.id = i.tournament_id
         WHERE i.id = $1",
    )
    .bind(invitation_id)
    .fetch_optional(pool)
    .await
    .map_err(InvitationError::Database)?;
    let row = authenticate(row, token, |row| &row.token_hash)?;
    let lifecycle = LockedInvitation {
        id: row.id,
        tournament_id: row.tournament_id,
        series_id: row.series_id,
        token_hash: row.token_hash,
        expires_at: row.expires_at,
        revoked_at: row.revoked_at,
        max_uses: row.max_uses,
        tournament_status: row.tournament_status,
    };
    check_lifecycle(&lifecycle, row.redemption_count, now)?;
    Ok(InvitationPreview {
        tournament: PreviewTournament {
            id: row.tournament_id,
            name: row.tournament_name,
            start_date: row.start_date,
            end_date: row.end_date,
        },
        invitation: PreviewInvitation {
            expires_at: row.expires_at,
        },
    })
}

#[derive(Debug, FromRow)]
struct InvitationIdentity {
    tournament_id: Uuid,
    token_hash: Vec<u8>,
}

pub(super) async fn authenticate_identity(
    pool: &PgPool,
    invitation_id: Uuid,
    token: &str,
) -> Result<Uuid, InvitationError> {
    let row = sqlx::query_as::<_, InvitationIdentity>(
        "SELECT tournament_id, token_hash
         FROM tournament_invitations WHERE id = $1",
    )
    .bind(invitation_id)
    .fetch_optional(pool)
    .await
    .map_err(InvitationError::Database)?;
    authenticate(row, token, |invitation| &invitation.token_hash)
        .map(|identity| identity.tournament_id)
}

pub async fn authenticate_token(
    pool: &PgPool,
    invitation_id: Uuid,
    token: &str,
) -> Result<(), InvitationError> {
    authenticate_identity(pool, invitation_id, token)
        .await
        .map(|_| ())
}
