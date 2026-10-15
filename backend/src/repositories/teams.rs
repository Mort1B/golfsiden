use std::collections::HashMap;

use chrono::NaiveTime;
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::models::{Team, TeamMember, TeamWithMembers},
    repositories::{
        session_read::{self, SessionReadError},
        tournament_authorization::{self, AuthorizationError},
    },
};

#[derive(Debug, Error)]
pub enum TeamMutationError {
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn list(pool: &PgPool, round_id: Uuid) -> Result<Vec<TeamWithMembers>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let result = list_in_transaction(&mut transaction, round_id).await?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn list_for_session(
    pool: &PgPool,
    session: Uuid,
    round_id: Uuid,
) -> Result<Vec<TeamWithMembers>, SessionReadError> {
    session_read::retry(|| async {
        let (mut tx, user) = session_read::begin(pool, session).await?;
        tournament_authorization::require_round_member_read(&mut tx, user, round_id).await?;
        let result = list_in_transaction(&mut tx, round_id).await?;
        session_read::finish(tx, session).await?;
        Ok(result)
    })
    .await
}

async fn list_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    round_id: Uuid,
) -> Result<Vec<TeamWithMembers>, sqlx::Error> {
    let teams = sqlx::query_as::<_, Team>("SELECT id, round_id, tournament_id, name, starting_hole, tee_time, created_at, updated_at FROM teams WHERE round_id = $1 ORDER BY name, id")
        .bind(round_id).fetch_all(&mut **transaction).await?;
    if teams.is_empty() {
        return Ok(Vec::new());
    }
    let team_ids: Vec<_> = teams.iter().map(|team| team.id).collect();
    let rows = sqlx::query_as::<_, ListedMember>("SELECT tm.team_id, tm.player_id, p.display_name, tm.display_order FROM team_memberships tm JOIN players p ON p.id = tm.player_id WHERE tm.team_id = ANY($1) ORDER BY tm.team_id, tm.display_order NULLS LAST, p.display_name, tm.player_id")
        .bind(&team_ids).fetch_all(&mut **transaction).await?;
    let mut members_by_team: HashMap<Uuid, Vec<TeamMember>> = HashMap::new();
    for row in rows {
        members_by_team
            .entry(row.team_id)
            .or_default()
            .push(TeamMember {
                player_id: row.player_id,
                display_name: row.display_name,
                display_order: row.display_order,
            });
    }
    Ok(teams
        .into_iter()
        .map(|team| TeamWithMembers {
            members: members_by_team.remove(&team.id).unwrap_or_default(),
            team,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ListedMember {
    team_id: Uuid,
    player_id: Uuid,
    display_name: String,
    display_order: Option<i16>,
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
