use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    auth::SessionPrincipal,
    domain::{
        models::{ScoringFormat, TournamentRole},
        round_formats::RoundFormatPolicy,
        scorecards::ScoreOwner,
    },
    repositories::auth,
};

#[derive(Debug, Error)]
pub enum ScoreAuthorizationError {
    #[error("resource not found")]
    NotFound,
    #[error("session is not authenticated")]
    Unauthenticated,
    #[error("session cannot write this scorecard")]
    Forbidden,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn writable_owners(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
) -> Result<Vec<ScoreOwner>, ScoreAuthorizationError> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *transaction)
        .await?;
    let principal = auth::lock_active_session(&mut transaction, session_id)
        .await?
        .ok_or(ScoreAuthorizationError::Unauthenticated)?;
    let context = round_context(&mut transaction, round_id).await?;
    let role = membership_role(&mut transaction, context.0, principal.user_id).await?;
    let owners = resolve_owners(&mut transaction, &principal, role, round_id, context.1).await?;
    transaction.commit().await?;
    Ok(owners)
}

pub async fn authorize_mutation(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    round_id: Uuid,
    format: ScoringFormat,
    owner: ScoreOwner,
) -> Result<Uuid, ScoreAuthorizationError> {
    let tournament_id =
        sqlx::query_scalar::<_, Uuid>("SELECT tournament_id FROM rounds WHERE id = $1")
            .bind(round_id)
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or(ScoreAuthorizationError::NotFound)?;
    let principal = auth::lock_active_session(transaction, session_id)
        .await?
        .ok_or(ScoreAuthorizationError::Unauthenticated)?;
    let role = membership_role(transaction, tournament_id, principal.user_id).await?;
    let owners = resolve_owners(transaction, &principal, role, round_id, format).await?;
    if owners.contains(&owner) {
        Ok(principal.user_id)
    } else {
        Err(ScoreAuthorizationError::Forbidden)
    }
}

async fn round_context(
    connection: &mut PgConnection,
    round_id: Uuid,
) -> Result<(Uuid, ScoringFormat), ScoreAuthorizationError> {
    sqlx::query_as("SELECT tournament_id, scoring_format FROM rounds WHERE id = $1")
        .bind(round_id)
        .fetch_optional(connection)
        .await?
        .ok_or(ScoreAuthorizationError::NotFound)
}

async fn resolve_owners(
    connection: &mut PgConnection,
    principal: &SessionPrincipal,
    role: Option<TournamentRole>,
    round_id: Uuid,
    format: ScoringFormat,
) -> Result<Vec<ScoreOwner>, ScoreAuthorizationError> {
    let privileged = matches!(role, Some(TournamentRole::Admin | TournamentRole::Scorer));
    let player_id = if role == Some(TournamentRole::Player) {
        principal.player_id
    } else {
        None
    };
    if !privileged && player_id.is_none() {
        return Ok(Vec::new());
    }

    match RoundFormatPolicy::for_format(format) {
        RoundFormatPolicy::PlayerOwned { .. } => {
            let ids = individual_owner_ids(connection, round_id, privileged, player_id).await?;
            Ok(ids
                .into_iter()
                .map(|id| ScoreOwner::Player { id })
                .collect())
        }
        RoundFormatPolicy::TeamOwned {
            exact_team_size, ..
        } => {
            let ids = team_owner_ids(
                connection,
                round_id,
                privileged,
                player_id,
                i64::from(exact_team_size),
                RoundFormatPolicy::for_format(format).requires_preserved_team_handicap_snapshot(),
            )
            .await?;
            Ok(ids.into_iter().map(|id| ScoreOwner::Team { id }).collect())
        }
    }
}

async fn individual_owner_ids(
    connection: &mut PgConnection,
    round_id: Uuid,
    privileged: bool,
    player_id: Option<Uuid>,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT rhs.player_id
         FROM round_handicap_snapshots rhs
         JOIN players p ON p.id = rhs.player_id
         WHERE rhs.round_id = $1
           AND (
             $2
             OR rhs.player_id = $3
             OR EXISTS (
               SELECT 1
               FROM flight_memberships actor_fm
               JOIN flight_memberships owner_fm
                 ON owner_fm.flight_id = actor_fm.flight_id
                AND owner_fm.round_id = actor_fm.round_id
                AND owner_fm.tournament_id = actor_fm.tournament_id
               WHERE actor_fm.round_id = $1
                 AND actor_fm.player_id = $3
                 AND owner_fm.player_id = rhs.player_id
             )
           )
         ORDER BY lower(p.display_name), p.id",
    )
    .bind(round_id)
    .bind(privileged)
    .bind(player_id)
    .fetch_all(connection)
    .await
}

async fn team_owner_ids(
    connection: &mut PgConnection,
    round_id: Uuid,
    privileged: bool,
    player_id: Option<Uuid>,
    exact_team_size: i64,
    requires_team_snapshot: bool,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT t.id
         FROM teams t
         WHERE t.round_id = $1
           AND (
             SELECT count(*) FROM team_memberships member_tm
             WHERE member_tm.round_id = t.round_id
               AND member_tm.team_id = t.id
           ) = $4
           AND (
             NOT $5
             OR EXISTS (
               SELECT 1 FROM round_team_handicap_snapshots rths
               WHERE rths.round_id = t.round_id AND rths.team_id = t.id
             )
           )
           AND (
             SELECT count(*)
             FROM team_memberships eligible_tm
             JOIN round_handicap_snapshots rhs
               ON rhs.round_id = eligible_tm.round_id
              AND rhs.player_id = eligible_tm.player_id
             WHERE eligible_tm.round_id = t.round_id
               AND eligible_tm.team_id = t.id
           ) = $4
           AND (
             $2
             OR EXISTS (
               SELECT 1 FROM team_memberships direct_tm
               WHERE direct_tm.round_id = t.round_id
                 AND direct_tm.team_id = t.id
                 AND direct_tm.player_id = $3
             )
             OR EXISTS (
               SELECT 1
               FROM flight_memberships actor_fm
               WHERE actor_fm.round_id = t.round_id
                 AND actor_fm.player_id = $3
                 AND NOT EXISTS (
                   SELECT 1
                   FROM team_memberships target_tm
                   WHERE target_tm.round_id = t.round_id
                     AND target_tm.team_id = t.id
                     AND NOT EXISTS (
                       SELECT 1
                       FROM flight_memberships target_fm
                       WHERE target_fm.flight_id = actor_fm.flight_id
                         AND target_fm.round_id = actor_fm.round_id
                         AND target_fm.tournament_id = actor_fm.tournament_id
                         AND target_fm.player_id = target_tm.player_id
                     )
                 )
             )
           )
         ORDER BY lower(t.name), t.id",
    )
    .bind(round_id)
    .bind(privileged)
    .bind(player_id)
    .bind(exact_team_size)
    .bind(requires_team_snapshot)
    .fetch_all(connection)
    .await
}

async fn membership_role(
    connection: &mut PgConnection,
    tournament_id: Uuid,
    user_id: Uuid,
) -> Result<Option<TournamentRole>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT role FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2 FOR SHARE",
    )
    .bind(tournament_id)
    .bind(user_id)
    .fetch_optional(connection)
    .await
}
