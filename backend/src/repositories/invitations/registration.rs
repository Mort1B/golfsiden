use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::invitations::ValidatedRegistration, repositories::auth};

use super::{
    InvitationError, RegisteredPlayer, authenticate, check_lifecycle, classify_database,
    lock_invitation, lock_series_and_count, writes,
};

pub struct RegisterParams<'a> {
    pub invitation_id: Uuid,
    pub token: &'a str,
    pub registration: &'a ValidatedRegistration,
    pub password_hash: &'a str,
    pub session_token_hash: &'a [u8],
    pub session_expires_at: DateTime<Utc>,
}

pub async fn register(
    pool: &PgPool,
    params: RegisterParams<'_>,
) -> Result<RegisteredPlayer, InvitationError> {
    let mut transaction = pool.begin().await.map_err(InvitationError::Database)?;
    let invitation = authenticate(
        lock_invitation(&mut transaction, params.invitation_id)
            .await
            .map_err(InvitationError::Database)?,
        params.token,
        |invitation| &invitation.token_hash,
    )?;
    let redemption_count = lock_series_and_count(&mut transaction, &invitation)
        .await
        .map_err(InvitationError::Database)?;
    let checked_at = super::transaction_clock(&mut transaction)
        .await
        .map_err(InvitationError::Database)?;
    check_lifecycle(&invitation, redemption_count, checked_at)?;

    let player_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO players (id, display_name, current_handicap_index)
         VALUES ($1, $2, $3)",
    )
    .bind(player_id)
    .bind(&params.registration.display_name)
    .bind(params.registration.handicap_index)
    .execute(&mut *transaction)
    .await
    .map_err(classify_database)?;
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, role, password_hash, player_id)
         VALUES ($1, $2, $3, 'player', $4, $5)",
    )
    .bind(user_id)
    .bind(&params.registration.username)
    .bind(&params.registration.display_name)
    .bind(params.password_hash)
    .bind(player_id)
    .execute(&mut *transaction)
    .await
    .map_err(classify_database)?;
    sqlx::query(
        "INSERT INTO handicap_history
           (id, player_id, handicap_index, changed_by, reason)
         VALUES ($1, $2, $3, $4, 'invitation registration')",
    )
    .bind(Uuid::new_v4())
    .bind(player_id)
    .bind(params.registration.handicap_index)
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    .map_err(classify_database)?;
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'player')",
    )
    .bind(invitation.tournament_id)
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    .map_err(classify_database)?;
    writes::insert_entrant(
        &mut transaction,
        invitation.tournament_id,
        player_id,
        user_id,
        params.registration.handicap_index,
    )
    .await?;
    writes::insert_redemption(
        &mut transaction,
        &invitation,
        user_id,
        player_id,
        "registration",
    )
    .await?;
    let session = auth::create_session_in_transaction(
        &mut transaction,
        user_id,
        params.session_token_hash,
        params.session_expires_at,
    )
    .await
    .map_err(classify_database)?;
    transaction.commit().await.map_err(classify_database)?;
    Ok(RegisteredPlayer {
        session,
        tournament_id: invitation.tournament_id,
        player_id,
    })
}
