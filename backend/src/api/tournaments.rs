use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    domain::models::{ScoringMode, Tournament, TournamentPlayer, TournamentStatus},
    error::{ApiError, ApiResult, require_non_empty},
    repositories::tournaments,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tournaments", get(list).post(create))
        .route("/api/tournaments/{tournament_id}", get(get_one))
        .route(
            "/api/tournaments/{tournament_id}/players",
            get(list_players).post(add_player),
        )
}

#[derive(Deserialize)]
struct CreateTournament {
    name: String,
    #[serde(default)]
    description: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
    number_of_rounds: i16,
    #[serde(default = "default_status")]
    status: TournamentStatus,
    #[serde(default = "default_scoring_mode")]
    scoring_mode: ScoringMode,
}

#[derive(Deserialize)]
struct AddTournamentPlayer {
    player_id: Uuid,
    tournament_handicap: Option<f64>,
    seed: Option<i16>,
}

fn default_status() -> TournamentStatus {
    TournamentStatus::Draft
}
fn default_scoring_mode() -> ScoringMode {
    ScoringMode::Combined
}

async fn list(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Tournament>>> {
    Ok(Json(tournaments::list(&state.pool).await?))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Tournament>> {
    Ok(Json(
        tournaments::get(&state.pool, id)
            .await?
            .ok_or(ApiError::NotFound)?,
    ))
}

async fn create(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateTournament>,
) -> ApiResult<impl IntoResponse> {
    require_non_empty(&input.name, "name")?;
    if input.end_date < input.start_date {
        return Err(ApiError::BadRequest(
            "end_date must not be before start_date".to_owned(),
        ));
    }
    if !(1..=30).contains(&input.number_of_rounds) {
        return Err(ApiError::BadRequest(
            "number_of_rounds must be between 1 and 30".to_owned(),
        ));
    }
    let tournament = tournaments::create(
        &state.pool,
        &input.name,
        &input.description,
        input.start_date,
        input.end_date,
        input.number_of_rounds,
        input.status,
        input.scoring_mode,
    )
    .await?;
    state.notify("tournament", tournament.id);
    Ok((StatusCode::CREATED, Json(tournament)))
}

async fn list_players(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<TournamentPlayer>>> {
    if tournaments::get(&state.pool, id).await?.is_none() {
        return Err(ApiError::NotFound);
    }
    Ok(Json(tournaments::list_players(&state.pool, id).await?))
}

async fn add_player(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(input): Json<AddTournamentPlayer>,
) -> ApiResult<impl IntoResponse> {
    if let Some(handicap) = input.tournament_handicap
        && (!handicap.is_finite() || !(-10.0..=54.0).contains(&handicap))
    {
        return Err(ApiError::BadRequest(
            "tournament_handicap must be between -10.0 and 54.0".to_owned(),
        ));
    }
    let player = tournaments::add_player(
        &state.pool,
        id,
        input.player_id,
        input.tournament_handicap,
        input.seed,
    )
    .await?;
    state.notify("tournament", id);
    Ok((StatusCode::CREATED, Json(player)))
}
