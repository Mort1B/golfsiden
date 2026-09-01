use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    domain::models::{ParticipantStatus, TournamentRole},
    repositories::auth,
};

use super::{
    AcceptedPlayer, InvitationError, JoinStatus, authenticate, check_lifecycle, classify_database,
    lock_invitation, lock_series_and_count, public::authenticate_identity, writes,
};

pub async fn accept(
    pool: &PgPool,
    session_id: Uuid,
    invitation_id: Uuid,
    token: &str,
) -> Result<AcceptedPlayer, InvitationError> {
    let tournament_id = authenticate_identity(pool, invitation_id, token).await?;
    let mut transaction = pool.begin().await.map_err(InvitationError::Database)?;
    let principal = auth::lock_active_session_exclusive(&mut transaction, session_id)
        .await
        .map_err(InvitationError::Database)?
        .ok_or(InvitationError::Unauthenticated)?;
    let player = lock_player(&mut transaction, principal.player_id).await?;
    let membership = sqlx::query_scalar::<_, TournamentRole>(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(tournament_id)
    .bind(principal.user_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(InvitationError::Database)?;
    let invitation = authenticate(
        lock_invitation(&mut transaction, invitation_id)
            .await
            .map_err(InvitationError::Database)?,
        token,
        |invitation| &invitation.token_hash,
    )?;
    let player_id = principal.player_id.ok_or(InvitationError::UnlinkedPlayer)?;
    let player = player.ok_or(InvitationError::UnlinkedPlayer)?;
    let redemption_count = lock_series_and_count(&mut transaction, &invitation)
        .await
        .map_err(InvitationError::Database)?;
    let entrant = sqlx::query_scalar::<_, ParticipantStatus>(
        "SELECT status FROM tournament_players
         WHERE tournament_id = $1 AND player_id = $2 FOR UPDATE",
    )
    .bind(invitation.tournament_id)
    .bind(player_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(InvitationError::Database)?;
    if !player.active {
        return Err(InvitationError::InactivePlayer);
    }
    if compatible(membership) && entrant == Some(ParticipantStatus::Active) {
        transaction.commit().await.map_err(classify_database)?;
        return Ok(AcceptedPlayer {
            status: JoinStatus::AlreadyJoined,
            tournament_id: invitation.tournament_id,
            player_id,
        });
    }
    let checked_at = super::transaction_clock(&mut transaction)
        .await
        .map_err(InvitationError::Database)?;
    check_lifecycle(&invitation, redemption_count, checked_at)?;
    if entrant == Some(ParticipantStatus::Withdrawn) {
        return Err(InvitationError::WithdrawnPlayer);
    }
    ensure_membership(
        &mut transaction,
        invitation.tournament_id,
        principal.user_id,
        membership,
    )
    .await?;
    if entrant.is_none() {
        writes::insert_entrant(
            &mut transaction,
            invitation.tournament_id,
            player_id,
            principal.user_id,
            player.handicap_index,
        )
        .await?;
    } else {
        writes::insert_missing_initial_handicap_history(
            &mut transaction,
            invitation.tournament_id,
            player_id,
            principal.user_id,
        )
        .await?;
    }
    writes::insert_redemption(
        &mut transaction,
        &invitation,
        principal.user_id,
        player_id,
        "acceptance",
    )
    .await?;
    transaction.commit().await.map_err(classify_database)?;
    Ok(AcceptedPlayer {
        status: JoinStatus::Joined,
        tournament_id: invitation.tournament_id,
        player_id,
    })
}

#[derive(Debug, FromRow)]
struct PlayerState {
    active: bool,
    handicap_index: f64,
}

async fn lock_player(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    player_id: Option<Uuid>,
) -> Result<Option<PlayerState>, InvitationError> {
    let Some(player_id) = player_id else {
        return Ok(None);
    };
    sqlx::query_as(
        "SELECT active, current_handicap_index::float8 AS handicap_index
         FROM players WHERE id = $1 FOR UPDATE",
    )
    .bind(player_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(InvitationError::Database)
}

fn compatible(role: Option<TournamentRole>) -> bool {
    matches!(
        role,
        Some(TournamentRole::Admin | TournamentRole::Scorer | TournamentRole::Player)
    )
}

async fn ensure_membership(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tournament_id: Uuid,
    user_id: Uuid,
    role: Option<TournamentRole>,
) -> Result<(), InvitationError> {
    match role {
        Some(TournamentRole::Viewer) => {
            sqlx::query(
                "UPDATE tournament_memberships SET role = 'player'
                 WHERE tournament_id = $1 AND user_id = $2",
            )
            .bind(tournament_id)
            .bind(user_id)
            .execute(&mut **transaction)
            .await
            .map_err(classify_database)?;
        }
        None => {
            sqlx::query(
                "INSERT INTO tournament_memberships (tournament_id, user_id, role)
                 VALUES ($1, $2, 'player')",
            )
            .bind(tournament_id)
            .bind(user_id)
            .execute(&mut **transaction)
            .await
            .map_err(classify_database)?;
        }
        Some(_) => {}
    }
    Ok(())
}
