use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{COLUMNS, TournamentMutationError};
use crate::{
    domain::models::{Tournament, TournamentStatus},
    repositories::tournament_authorization,
};

pub struct UpdateCountedRoundsResult {
    pub tournament: Tournament,
    pub changed: bool,
}

pub async fn update_counted_rounds_authorized(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    counted_rounds: i16,
    mandatory_round_id: Option<Uuid>,
    expected_updated_at: DateTime<Utc>,
) -> Result<UpdateCountedRoundsResult, TournamentMutationError> {
    let likely_admin = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1
           FROM user_sessions s
           JOIN tournament_memberships tm ON tm.user_id = s.user_id
           WHERE s.id = $1
             AND s.revoked_at IS NULL
             AND s.expires_at > now()
             AND tm.tournament_id = $2
             AND tm.role = 'admin'
         )",
    )
    .bind(session_id)
    .bind(tournament_id)
    .fetch_one(pool)
    .await?;
    if !likely_admin {
        let mut authorization = pool.begin().await?;
        tournament_authorization::require_tournament_admin(
            &mut authorization,
            session_id,
            tournament_id,
        )
        .await?;
        authorization.rollback().await?;
    }

    let mut transaction = pool.begin().await?;

    let round_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM rounds
         WHERE tournament_id = $1
         ORDER BY id
         FOR UPDATE",
    )
    .bind(tournament_id)
    .fetch_all(&mut *transaction)
    .await?;

    let actor = tournament_authorization::require_tournament_admin(
        &mut transaction,
        session_id,
        tournament_id,
    )
    .await?;

    let tournament = sqlx::query_as::<_, Tournament>(&format!(
        "SELECT {COLUMNS} FROM tournaments WHERE id = $1 FOR UPDATE"
    ))
    .bind(tournament_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(TournamentMutationError::NotFound)?;

    if counted_rounds < 1 || counted_rounds > tournament.number_of_rounds {
        return Err(TournamentMutationError::CountedRoundsInvalid);
    }
    if mandatory_round_id.is_some_and(|id| !round_ids.contains(&id)) {
        return Err(TournamentMutationError::MandatoryRoundInvalid);
    }
    let locked = tournament.status != TournamentStatus::Draft
        || sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                 SELECT 1 FROM tournament_handicap_locks WHERE tournament_id = $1
             ) OR EXISTS (
                 SELECT 1 FROM rounds WHERE tournament_id = $1 AND status <> 'draft'
             ) OR EXISTS (
                 SELECT 1 FROM round_handicap_snapshots WHERE tournament_id = $1
             ) OR EXISTS (
                 SELECT 1 FROM round_team_handicap_snapshots WHERE tournament_id = $1
             )",
        )
        .bind(tournament_id)
        .fetch_one(&mut *transaction)
        .await?;
    if locked {
        return Err(TournamentMutationError::ConfigurationLocked);
    }
    if tournament.updated_at != expected_updated_at {
        return Err(TournamentMutationError::ConfigurationStale);
    }
    if tournament.counted_rounds == counted_rounds
        && tournament.mandatory_round_id == mandatory_round_id
    {
        transaction.commit().await?;
        return Ok(UpdateCountedRoundsResult {
            tournament,
            changed: false,
        });
    }

    sqlx::query(
        "SELECT
           set_config('app.tournament_configuration_tournament_id', $1::text, true),
           set_config('app.tournament_configuration_user_id', $2::text, true)",
    )
    .bind(tournament_id)
    .bind(actor)
    .execute(&mut *transaction)
    .await?;

    let tournament = sqlx::query_as::<_, Tournament>(&format!(
        "UPDATE tournaments SET counted_rounds = $2, mandatory_round_id = $3
         WHERE id = $1
         RETURNING {COLUMNS}"
    ))
    .bind(tournament_id)
    .bind(counted_rounds)
    .bind(mandatory_round_id)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(UpdateCountedRoundsResult {
        tournament,
        changed: true,
    })
}
