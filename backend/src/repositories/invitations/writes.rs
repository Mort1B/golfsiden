use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{InvitationError, LockedInvitation, classify_database};

pub(super) async fn insert_entrant(
    transaction: &mut Transaction<'_, Postgres>,
    tournament_id: Uuid,
    player_id: Uuid,
    user_id: Uuid,
    handicap_index: f64,
) -> Result<(), InvitationError> {
    sqlx::query(
        "INSERT INTO tournament_players
           (tournament_id, player_id, tournament_handicap)
         VALUES ($1, $2, $3)",
    )
    .bind(tournament_id)
    .bind(player_id)
    .bind(handicap_index)
    .execute(&mut **transaction)
    .await
    .map_err(classify_database)?;
    sqlx::query(
        "INSERT INTO tournament_handicap_history
           (id, tournament_id, player_id, handicap_index, changed_by, reason)
         VALUES ($1, $2, $3, $4, $5, 'invitation join snapshot')",
    )
    .bind(Uuid::new_v4())
    .bind(tournament_id)
    .bind(player_id)
    .bind(handicap_index)
    .bind(user_id)
    .execute(&mut **transaction)
    .await
    .map_err(classify_database)?;
    Ok(())
}

pub(super) async fn insert_missing_initial_handicap_history(
    transaction: &mut Transaction<'_, Postgres>,
    tournament_id: Uuid,
    player_id: Uuid,
    user_id: Uuid,
) -> Result<(), InvitationError> {
    // Acceptance holds the entrant row FOR UPDATE, so concurrent repairs cannot
    // both observe a missing initial history before either transaction commits.
    sqlx::query(
        "INSERT INTO tournament_handicap_history
           (id, tournament_id, player_id, handicap_index, changed_by, reason)
         SELECT $1, tp.tournament_id, tp.player_id, tp.tournament_handicap,
                $4, 'invitation acceptance initial handicap repair'
         FROM tournament_players tp
         WHERE tp.tournament_id = $2 AND tp.player_id = $3
           AND NOT EXISTS (
             SELECT 1 FROM tournament_handicap_history history
             WHERE history.tournament_id = tp.tournament_id
               AND history.player_id = tp.player_id
           )",
    )
    .bind(Uuid::new_v4())
    .bind(tournament_id)
    .bind(player_id)
    .bind(user_id)
    .execute(&mut **transaction)
    .await
    .map_err(classify_database)?;
    Ok(())
}

pub(super) async fn insert_redemption(
    transaction: &mut Transaction<'_, Postgres>,
    invitation: &LockedInvitation,
    user_id: Uuid,
    player_id: Uuid,
    mode: &str,
) -> Result<(), InvitationError> {
    sqlx::query(
        "INSERT INTO invitation_redemptions
           (id, invitation_id, series_id, tournament_id, user_id, player_id, mode)
         VALUES ($1, $2, $3, $4, $5, $6, $7::invitation_redemption_mode)",
    )
    .bind(Uuid::new_v4())
    .bind(invitation.id)
    .bind(invitation.series_id)
    .bind(invitation.tournament_id)
    .bind(user_id)
    .bind(player_id)
    .bind(mode)
    .execute(&mut **transaction)
    .await
    .map_err(classify_database)?;
    Ok(())
}
