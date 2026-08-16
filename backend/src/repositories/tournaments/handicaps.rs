use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::models::{
        TournamentHandicapCorrection, TournamentHandicapCorrectionState,
        TournamentHandicapHistoryEntry, TournamentHandicapLockReason, TournamentPlayer,
        TournamentPlayerRoster,
    },
    repositories::tournament_authorization::{self, AuthorizationError},
};

use super::TournamentMutationError;

pub async fn list_players(
    pool: &PgPool,
    tournament_id: Uuid,
) -> Result<TournamentPlayerRoster, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *transaction)
        .await?;
    let roster = list_players_in_transaction(&mut transaction, tournament_id).await?;
    transaction.commit().await?;
    Ok(roster)
}

pub async fn list_players_for_member(
    pool: &PgPool,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<TournamentPlayerRoster, AuthorizationError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    tournament_authorization::require_tournament_member_read(
        &mut transaction,
        user_id,
        tournament_id,
    )
    .await?;
    let roster = list_players_in_transaction(&mut transaction, tournament_id).await?;
    transaction.commit().await?;
    Ok(roster)
}

async fn list_players_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    tournament_id: Uuid,
) -> Result<TournamentPlayerRoster, sqlx::Error> {
    let players = sqlx::query_as::<_, TournamentPlayer>(
        "SELECT tp.tournament_id, tp.player_id, p.display_name,
                tp.tournament_handicap::float8 AS tournament_handicap,
                tp.seed, tp.status, tp.created_at, tp.updated_at
         FROM tournament_players tp
         JOIN players p ON p.id = tp.player_id
         WHERE tp.tournament_id = $1
         ORDER BY tp.seed NULLS LAST, p.display_name, tp.player_id",
    )
    .bind(tournament_id)
    .fetch_all(&mut **transaction)
    .await?;
    let lock_reason = sqlx::query_scalar::<_, TournamentHandicapLockReason>(
        "SELECT reason FROM tournament_handicap_locks WHERE tournament_id = $1",
    )
    .bind(tournament_id)
    .fetch_optional(&mut **transaction)
    .await?;
    let handicap_correction = match lock_reason {
        Some(reason) => TournamentHandicapCorrectionState::Locked { reason },
        None => TournamentHandicapCorrectionState::Editable,
    };
    let roster = TournamentPlayerRoster {
        handicap_correction,
        players,
    };
    Ok(roster)
}

pub async fn change_player_handicap_authorized(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    player_id: Uuid,
    handicap: f64,
    reason: &str,
) -> Result<TournamentHandicapCorrection, TournamentMutationError> {
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "SELECT id FROM rounds
         WHERE tournament_id = $1
         ORDER BY id
         FOR UPDATE",
    )
    .bind(tournament_id)
    .execute(&mut *transaction)
    .await?;
    let actor = tournament_authorization::require_tournament_admin(
        &mut transaction,
        session_id,
        tournament_id,
    )
    .await?;
    sqlx::query("SELECT id FROM tournaments WHERE id = $1 FOR UPDATE")
        .bind(tournament_id)
        .execute(&mut *transaction)
        .await?;
    let audit_id = Uuid::new_v4();
    sqlx::query(
        "SELECT
           set_config('app.tournament_handicap_correction_tournament_id', $1::text, true),
           set_config('app.tournament_handicap_correction_player_id', $2::text, true),
           set_config('app.tournament_handicap_correction_user_id', $3::text, true),
           set_config('app.tournament_handicap_correction_audit_id', $4::text, true),
           set_config('app.tournament_handicap_correction_reason', $5, true)",
    )
    .bind(tournament_id)
    .bind(player_id)
    .bind(actor)
    .bind(audit_id)
    .bind(reason.trim())
    .execute(&mut *transaction)
    .await?;
    let player = sqlx::query_as::<_, TournamentPlayer>(
        "UPDATE tournament_players AS tp SET tournament_handicap = $3
         WHERE tp.tournament_id = $1 AND tp.player_id = $2
         RETURNING tp.tournament_id, tp.player_id,
           (SELECT p.display_name FROM players p WHERE p.id = tp.player_id) AS display_name,
           tp.tournament_handicap::float8 AS tournament_handicap,
           tp.seed, tp.status, tp.created_at, tp.updated_at",
    )
    .bind(tournament_id)
    .bind(player_id)
    .bind(handicap)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(classify_mutation_error)?
    .ok_or(TournamentMutationError::NotFound)?;
    let audit = sqlx::query_as::<_, TournamentHandicapHistoryEntry>(
        "SELECT id, tournament_id, player_id,
                handicap_index::float8 AS handicap_index, effective_from,
                changed_by, reason, created_at
         FROM tournament_handicap_history WHERE id = $1",
    )
    .bind(audit_id)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(TournamentHandicapCorrection { player, audit })
}

fn classify_mutation_error(error: sqlx::Error) -> TournamentMutationError {
    if let sqlx::Error::Database(database) = &error {
        match database.constraint() {
            Some("tournament_handicap_locked") => {
                return TournamentMutationError::HandicapLocked;
            }
            Some("tournament_handicap_unchanged") => {
                return TournamentMutationError::HandicapUnchanged;
            }
            _ => {}
        }
    }
    TournamentMutationError::Database(error)
}
