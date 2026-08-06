use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::models::{ScoringMode, Tournament, TournamentPlayer, TournamentStatus};

const COLUMNS: &str = "id, name, description, start_date, end_date, number_of_rounds, status, scoring_mode, created_at, updated_at";

pub async fn list(pool: &PgPool) -> Result<Vec<Tournament>, sqlx::Error> {
    sqlx::query_as::<_, Tournament>(&format!(
        "SELECT {COLUMNS} FROM tournaments ORDER BY start_date DESC, name"
    ))
    .fetch_all(pool)
    .await
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<Tournament>, sqlx::Error> {
    sqlx::query_as::<_, Tournament>(&format!("SELECT {COLUMNS} FROM tournaments WHERE id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    pool: &PgPool,
    name: &str,
    description: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    number_of_rounds: i16,
    status: TournamentStatus,
    scoring_mode: ScoringMode,
) -> Result<Tournament, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments (id, name, description, start_date, end_date, number_of_rounds, status, scoring_mode) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(id).bind(name.trim()).bind(description.trim()).bind(start_date).bind(end_date)
        .bind(number_of_rounds).bind(status).bind(scoring_mode).execute(pool).await?;
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn list_players(
    pool: &PgPool,
    tournament_id: Uuid,
) -> Result<Vec<TournamentPlayer>, sqlx::Error> {
    sqlx::query_as::<_, TournamentPlayer>("SELECT tp.tournament_id, tp.player_id, p.display_name, tp.tournament_handicap::float8 AS tournament_handicap, tp.seed, tp.status, tp.created_at, tp.updated_at FROM tournament_players tp JOIN players p ON p.id = tp.player_id WHERE tp.tournament_id = $1 ORDER BY tp.seed NULLS LAST, p.display_name")
        .bind(tournament_id).fetch_all(pool).await
}

pub async fn add_player(
    pool: &PgPool,
    tournament_id: Uuid,
    player_id: Uuid,
    handicap: Option<f64>,
    seed: Option<i16>,
) -> Result<TournamentPlayer, sqlx::Error> {
    sqlx::query("INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap, seed) SELECT $1, id, COALESCE($3, current_handicap_index), $4 FROM players WHERE id = $2")
        .bind(tournament_id).bind(player_id).bind(handicap).bind(seed).execute(pool).await?;
    sqlx::query_as::<_, TournamentPlayer>("SELECT tp.tournament_id, tp.player_id, p.display_name, tp.tournament_handicap::float8 AS tournament_handicap, tp.seed, tp.status, tp.created_at, tp.updated_at FROM tournament_players tp JOIN players p ON p.id = tp.player_id WHERE tp.tournament_id = $1 AND tp.player_id = $2")
        .bind(tournament_id).bind(player_id).fetch_one(pool).await
}
