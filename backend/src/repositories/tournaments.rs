mod handicaps;

pub use handicaps::{change_player_handicap_authorized, list_players};

use chrono::NaiveDate;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::models::{
        MyTournament, ScoringMode, Tournament, TournamentPlayer, TournamentRole, TournamentStatus,
    },
    repositories::tournament_authorization::{self, AuthorizationError},
};

const COLUMNS: &str = "id, name, description, start_date, end_date, number_of_rounds, status, scoring_mode, created_at, updated_at";

#[derive(Debug, Error)]
pub enum TournamentMutationError {
    #[error("resource not found")]
    NotFound,
    #[error("tournament handicap is locked")]
    HandicapLocked,
    #[error("tournament handicap is unchanged")]
    HandicapUnchanged,
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

#[derive(FromRow)]
struct MyTournamentRow {
    id: Uuid,
    name: String,
    description: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
    number_of_rounds: i16,
    status: TournamentStatus,
    scoring_mode: ScoringMode,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    role: TournamentRole,
    player_id: Option<Uuid>,
}

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

#[allow(clippy::too_many_arguments)]
pub async fn create_platform_authorized(
    pool: &PgPool,
    session_id: Uuid,
    name: &str,
    description: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    number_of_rounds: i16,
    status: TournamentStatus,
    scoring_mode: ScoringMode,
) -> Result<Tournament, TournamentMutationError> {
    let mut transaction = pool.begin().await?;
    let actor =
        tournament_authorization::require_platform_admin(&mut transaction, session_id).await?;
    let tournament = insert_tournament(
        &mut transaction,
        name,
        description,
        start_date,
        end_date,
        number_of_rounds,
        status,
        scoring_mode,
    )
    .await?;
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(tournament.id)
    .bind(actor)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(tournament)
}

pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<MyTournament>, sqlx::Error> {
    let rows = sqlx::query_as::<_, MyTournamentRow>(
        "SELECT t.id, t.name, t.description, t.start_date, t.end_date,
                t.number_of_rounds, t.status, t.scoring_mode, t.created_at,
                t.updated_at, tm.role,
                CASE WHEN tp.player_id IS NOT NULL THEN u.player_id END AS player_id
         FROM tournament_memberships tm
         JOIN tournaments t ON t.id = tm.tournament_id
         JOIN users u ON u.id = tm.user_id
         LEFT JOIN tournament_players tp
           ON tp.tournament_id = tm.tournament_id AND tp.player_id = u.player_id
         WHERE tm.user_id = $1
         ORDER BY t.start_date DESC, lower(t.name), t.id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(MyTournamentRow::into_model).collect())
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

pub async fn add_player_authorized(
    pool: &PgPool,
    session_id: Uuid,
    tournament_id: Uuid,
    player_id: Uuid,
    handicap: Option<f64>,
    seed: Option<i16>,
) -> Result<TournamentPlayer, TournamentMutationError> {
    let mut transaction = pool.begin().await?;
    let actor = tournament_authorization::require_tournament_admin(
        &mut transaction,
        session_id,
        tournament_id,
    )
    .await?;
    let player = insert_player(
        &mut transaction,
        tournament_id,
        player_id,
        handicap,
        seed,
        actor,
    )
    .await?;
    transaction.commit().await?;
    Ok(player)
}

#[allow(clippy::too_many_arguments)]
async fn insert_tournament(
    transaction: &mut Transaction<'_, Postgres>,
    name: &str,
    description: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    number_of_rounds: i16,
    status: TournamentStatus,
    scoring_mode: ScoringMode,
) -> Result<Tournament, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query_as::<_, Tournament>(&format!(
        "INSERT INTO tournaments (id, name, description, start_date, end_date, number_of_rounds, status, scoring_mode)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(name.trim())
    .bind(description.trim())
    .bind(start_date)
    .bind(end_date)
    .bind(number_of_rounds)
    .bind(status)
    .bind(scoring_mode)
    .fetch_one(&mut **transaction)
    .await
}

async fn insert_player(
    transaction: &mut Transaction<'_, Postgres>,
    tournament_id: Uuid,
    player_id: Uuid,
    handicap: Option<f64>,
    seed: Option<i16>,
    actor: Uuid,
) -> Result<TournamentPlayer, sqlx::Error> {
    let handicap = sqlx::query_scalar::<_, f64>(
        "INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap, seed)
         SELECT $1, id, COALESCE($3, current_handicap_index), $4
         FROM players WHERE id = $2
         RETURNING tournament_handicap::float8",
    )
    .bind(tournament_id)
    .bind(player_id)
    .bind(handicap)
    .bind(seed)
    .fetch_one(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO tournament_handicap_history
           (id, tournament_id, player_id, handicap_index, changed_by, reason)
         VALUES ($1, $2, $3, $4, $5, 'initial tournament handicap')",
    )
    .bind(Uuid::new_v4())
    .bind(tournament_id)
    .bind(player_id)
    .bind(handicap)
    .bind(actor)
    .execute(&mut **transaction)
    .await?;
    sqlx::query_as::<_, TournamentPlayer>(
        "SELECT tp.tournament_id, tp.player_id, p.display_name,
                tp.tournament_handicap::float8 AS tournament_handicap,
                tp.seed, tp.status, tp.created_at, tp.updated_at
         FROM tournament_players tp JOIN players p ON p.id = tp.player_id
         WHERE tp.tournament_id = $1 AND tp.player_id = $2",
    )
    .bind(tournament_id)
    .bind(player_id)
    .fetch_one(&mut **transaction)
    .await
}

impl MyTournamentRow {
    fn into_model(self) -> MyTournament {
        MyTournament {
            tournament: Tournament {
                id: self.id,
                name: self.name,
                description: self.description,
                start_date: self.start_date,
                end_date: self.end_date,
                number_of_rounds: self.number_of_rounds,
                status: self.status,
                scoring_mode: self.scoring_mode,
                created_at: self.created_at,
                updated_at: self.updated_at,
            },
            role: self.role,
            player_id: self.player_id,
        }
    }
}
