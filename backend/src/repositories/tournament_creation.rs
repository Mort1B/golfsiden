use super::{
    auth,
    tournament_authorization::{self, AuthorizationError},
    tournament_plan,
};
use crate::domain::tournament_plan::ValidatedTournamentPlan;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CreationError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("creation request key was used for different input")]
    ChangedRequest,
    #[error("tournament end date is in the past")]
    PastEndDate,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
pub struct CreatedTournament {
    pub tournament_id: Uuid,
    pub created: bool,
}

pub async fn create(
    pool: &PgPool,
    session_id: Uuid,
    request_id: Uuid,
    input: &ValidatedTournamentPlan,
) -> Result<CreatedTournament, CreationError> {
    let normalized = serde_json::to_vec(input)
        .map_err(|_| sqlx::Error::Protocol("could not encode validated plan".into()))?;
    let hash = Sha256::digest(normalized).to_vec();
    let mut tx = pool.begin().await?;
    // Locks both session and user, including requests from different sessions.
    let principal = auth::lock_active_session_exclusive(&mut tx, session_id)
        .await?
        .ok_or(AuthorizationError::Unauthenticated)?;
    let previous = sqlx::query_as::<_, (Vec<u8>,Uuid)>(
        "SELECT request_hash, tournament_id FROM tournament_creation_requests WHERE user_id=$1 AND request_id=$2"
    ).bind(principal.user_id).bind(request_id).fetch_optional(&mut *tx).await?;
    if let Some((previous_hash, tournament_id)) = previous {
        tournament_authorization::require_tournament_admin_read_in_transaction(
            &mut tx,
            principal.user_id,
            tournament_id,
        )
        .await?;
        require_live_session(&mut tx, session_id).await?;
        if previous_hash != hash {
            return Err(CreationError::ChangedRequest);
        }
        tx.commit().await?;
        return Ok(CreatedTournament {
            tournament_id,
            created: false,
        });
    }
    let entrant = if let Some(player_id) = principal.player_id {
        sqlx::query_scalar::<_, f64>(
            "SELECT current_handicap_index::float8 FROM players WHERE id=$1 AND active FOR SHARE",
        )
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await?
        .map(|handicap| (player_id, handicap))
    } else {
        None
    };
    require_live_session(&mut tx, session_id).await?;
    let eligible = sqlx::query_scalar::<_, bool>(
        "SELECT $1::date >= (clock_timestamp() AT TIME ZONE 'UTC')::date",
    )
    .bind(input.end_date)
    .fetch_one(&mut *tx)
    .await?;
    if !eligible {
        return Err(CreationError::PastEndDate);
    }
    let (tournament, _) =
        tournament_plan::insert(&mut tx, principal.user_id, entrant, input).await?;
    sqlx::query("INSERT INTO tournament_creation_requests (user_id, request_id, request_hash, tournament_id) VALUES ($1,$2,$3,$4)")
        .bind(principal.user_id).bind(request_id).bind(hash).bind(tournament.id).execute(&mut *tx).await?;
    // Also covers unexpected waits during dependent writes.
    require_live_session(&mut tx, session_id).await?;
    tx.commit().await?;
    Ok(CreatedTournament {
        tournament_id: tournament.id,
        created: true,
    })
}
async fn require_live_session(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session_id: Uuid,
) -> Result<(), CreationError> {
    let live = sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM user_sessions WHERE id=$1 AND revoked_at IS NULL AND expires_at > clock_timestamp())")
        .bind(session_id).fetch_one(&mut **tx).await?;
    if !live {
        return Err(AuthorizationError::Unauthenticated.into());
    }
    Ok(())
}
