use chrono::NaiveTime;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::models::{Team, TeamMember, TeamWithMembers},
    repositories::tournament_authorization::{self, AuthorizationError},
};

#[derive(Debug, Error)]
pub enum TeamMutationError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn list(pool: &PgPool, round_id: Uuid) -> Result<Vec<TeamWithMembers>, sqlx::Error> {
    let teams = sqlx::query_as::<_, Team>("SELECT id, round_id, tournament_id, name, starting_hole, tee_time, created_at, updated_at FROM teams WHERE round_id = $1 ORDER BY name")
        .bind(round_id).fetch_all(pool).await?;
    let mut result = Vec::with_capacity(teams.len());
    for team in teams {
        let members = sqlx::query_as::<_, TeamMember>("SELECT tm.player_id, p.display_name, tm.display_order FROM team_memberships tm JOIN players p ON p.id = tm.player_id WHERE tm.team_id = $1 ORDER BY tm.display_order NULLS LAST, p.display_name")
            .bind(team.id).fetch_all(pool).await?;
        result.push(TeamWithMembers { team, members });
    }
    Ok(result)
}

pub async fn create(
    pool: &PgPool,
    round_id: Uuid,
    name: &str,
    starting_hole: Option<i16>,
    tee_time: Option<NaiveTime>,
) -> Result<TeamWithMembers, sqlx::Error> {
    let id = Uuid::new_v4();
    let team = sqlx::query_as::<_, Team>("INSERT INTO teams (id, round_id, tournament_id, name, starting_hole, tee_time) SELECT $1, id, tournament_id, $3, $4, $5 FROM rounds WHERE id = $2 RETURNING id, round_id, tournament_id, name, starting_hole, tee_time, created_at, updated_at")
        .bind(id).bind(round_id).bind(name.trim()).bind(starting_hole).bind(tee_time).fetch_one(pool).await?;
    Ok(TeamWithMembers {
        team,
        members: vec![],
    })
}

pub async fn create_authorized(
    pool: &PgPool,
    session_id: Uuid,
    round_id: Uuid,
    name: &str,
    starting_hole: Option<i16>,
    tee_time: Option<NaiveTime>,
) -> Result<TeamWithMembers, TeamMutationError> {
    let mut transaction = pool.begin().await?;
    tournament_authorization::require_round_admin(&mut transaction, session_id, round_id).await?;
    let id = Uuid::new_v4();
    let team = sqlx::query_as::<_, Team>(
        "INSERT INTO teams (id, round_id, tournament_id, name, starting_hole, tee_time)
         SELECT $1, id, tournament_id, $3, $4, $5 FROM rounds WHERE id = $2
         RETURNING id, round_id, tournament_id, name, starting_hole, tee_time, created_at, updated_at",
    )
    .bind(id)
    .bind(round_id)
    .bind(name.trim())
    .bind(starting_hole)
    .bind(tee_time)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(TeamWithMembers {
        team,
        members: Vec::new(),
    })
}

pub async fn assign_player(
    pool: &PgPool,
    team_id: Uuid,
    player_id: Uuid,
    display_order: Option<i16>,
) -> Result<TeamMember, sqlx::Error> {
    sqlx::query("INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order) SELECT id, round_id, tournament_id, $2, $3 FROM teams WHERE id = $1")
        .bind(team_id).bind(player_id).bind(display_order).execute(pool).await?;
    sqlx::query_as::<_, TeamMember>("SELECT tm.player_id, p.display_name, tm.display_order FROM team_memberships tm JOIN players p ON p.id = tm.player_id WHERE tm.team_id = $1 AND tm.player_id = $2")
        .bind(team_id).bind(player_id).fetch_one(pool).await
}

pub async fn assign_player_authorized(
    pool: &PgPool,
    session_id: Uuid,
    team_id: Uuid,
    player_id: Uuid,
    display_order: Option<i16>,
) -> Result<TeamMember, TeamMutationError> {
    let mut transaction = pool.begin().await?;
    tournament_authorization::require_team_admin(&mut transaction, session_id, team_id).await?;
    sqlx::query(
        "INSERT INTO team_memberships
           (team_id, round_id, tournament_id, player_id, display_order)
         SELECT id, round_id, tournament_id, $2, $3 FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .bind(player_id)
    .bind(display_order)
    .execute(&mut *transaction)
    .await?;
    let member = sqlx::query_as::<_, TeamMember>(
        "SELECT tm.player_id, p.display_name, tm.display_order
         FROM team_memberships tm JOIN players p ON p.id = tm.player_id
         WHERE tm.team_id = $1 AND tm.player_id = $2",
    )
    .bind(team_id)
    .bind(player_id)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(member)
}

pub async fn remove_player(
    pool: &PgPool,
    team_id: Uuid,
    player_id: Uuid,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("DELETE FROM team_memberships WHERE team_id = $1 AND player_id = $2")
            .bind(team_id)
            .bind(player_id)
            .execute(pool)
            .await?
            .rows_affected()
            == 1,
    )
}

pub async fn remove_player_authorized(
    pool: &PgPool,
    session_id: Uuid,
    team_id: Uuid,
    player_id: Uuid,
) -> Result<bool, TeamMutationError> {
    let mut transaction = pool.begin().await?;
    tournament_authorization::require_team_admin(&mut transaction, session_id, team_id).await?;
    let removed = sqlx::query("DELETE FROM team_memberships WHERE team_id = $1 AND player_id = $2")
        .bind(team_id)
        .bind(player_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
        == 1;
    transaction.commit().await?;
    Ok(removed)
}
