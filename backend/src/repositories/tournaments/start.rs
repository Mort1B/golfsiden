use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{COLUMNS, TournamentMutationError};
use crate::{
    domain::models::{ParticipantStatus, Tournament, TournamentStatus},
    repositories::tournament_authorization,
};

pub struct StartTournamentResult {
    pub tournament: Tournament,
    pub changed: bool,
}

pub async fn start_authorized(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    expected_updated_at: DateTime<Utc>,
) -> Result<StartTournamentResult, TournamentMutationError> {
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
    let round_rows = sqlx::query_as::<_, (i16, crate::domain::models::RoundStatus)>(
        "SELECT round_number, status
         FROM rounds
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

    if tournament.status == TournamentStatus::Active {
        transaction.commit().await?;
        return Ok(StartTournamentResult {
            tournament,
            changed: false,
        });
    }
    if tournament.status != TournamentStatus::Draft {
        return Err(TournamentMutationError::StartInvalidState);
    }
    if tournament.updated_at != expected_updated_at {
        return Err(TournamentMutationError::StartStale);
    }

    let expected_rounds = usize::try_from(tournament.number_of_rounds)
        .map_err(|_| TournamentMutationError::StartNotReady)?;
    let mut round_numbers = round_rows.iter().map(|row| row.0).collect::<Vec<_>>();
    round_numbers.sort_unstable();
    let round_plan_ready = round_rows.len() == expected_rounds
        && round_rows
            .iter()
            .all(|row| row.1 == crate::domain::models::RoundStatus::Draft)
        && round_numbers
            .iter()
            .enumerate()
            .all(|(index, number)| usize::try_from(*number).ok() == Some(index + 1));
    if !round_plan_ready {
        return Err(TournamentMutationError::StartNotReady);
    }

    let entrants = sqlx::query_as::<_, (ParticipantStatus, bool)>(
        "SELECT tp.status, p.active
         FROM tournament_players tp
         JOIN players p ON p.id = tp.player_id
         WHERE tp.tournament_id = $1
         ORDER BY tp.player_id
         FOR SHARE OF tp, p",
    )
    .bind(tournament_id)
    .fetch_all(&mut *transaction)
    .await?;
    if !entrants
        .iter()
        .any(|(status, active)| *status == ParticipantStatus::Active && *active)
    {
        return Err(TournamentMutationError::StartNotReady);
    }

    sqlx::query(
        "SELECT
           set_config('app.tournament_start_tournament_id', $1::text, true),
           set_config('app.tournament_start_user_id', $2::text, true)",
    )
    .bind(tournament_id)
    .bind(actor)
    .execute(&mut *transaction)
    .await?;
    let tournament = sqlx::query_as::<_, Tournament>(&format!(
        "UPDATE tournaments SET status = 'active'
         WHERE id = $1
         RETURNING {COLUMNS}"
    ))
    .bind(tournament_id)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(StartTournamentResult {
        tournament,
        changed: true,
    })
}
