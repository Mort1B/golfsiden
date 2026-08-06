use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use uuid::Uuid;

use crate::{
    AppState,
    domain::leaderboards::{LeaderboardMetric, RoundLeaderboard, TournamentLeaderboard},
    error::{ApiError, ApiResult},
    repositories::leaderboards::{self, LeaderboardError},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/rounds/{round_id}/leaderboards/gross",
            get(round_gross),
        )
        .route("/api/rounds/{round_id}/leaderboards/net", get(round_net))
        .route(
            "/api/tournaments/{tournament_id}/leaderboards/gross",
            get(tournament_gross),
        )
        .route(
            "/api/tournaments/{tournament_id}/leaderboards/net",
            get(tournament_net),
        )
}

async fn round_gross(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<RoundLeaderboard>> {
    round(&state, round_id, LeaderboardMetric::Gross).await
}

async fn round_net(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<RoundLeaderboard>> {
    round(&state, round_id, LeaderboardMetric::Net).await
}

async fn tournament_gross(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
) -> ApiResult<Json<TournamentLeaderboard>> {
    tournament(&state, tournament_id, LeaderboardMetric::Gross).await
}

async fn tournament_net(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
) -> ApiResult<Json<TournamentLeaderboard>> {
    tournament(&state, tournament_id, LeaderboardMetric::Net).await
}

async fn round(
    state: &AppState,
    round_id: Uuid,
    metric: LeaderboardMetric,
) -> ApiResult<Json<RoundLeaderboard>> {
    Ok(Json(
        leaderboards::round(&state.pool, round_id, metric)
            .await
            .map_err(map_error)?,
    ))
}

async fn tournament(
    state: &AppState,
    tournament_id: Uuid,
    metric: LeaderboardMetric,
) -> ApiResult<Json<TournamentLeaderboard>> {
    Ok(Json(
        leaderboards::tournament(&state.pool, tournament_id, metric)
            .await
            .map_err(map_error)?,
    ))
}

fn map_error(error: LeaderboardError) -> ApiError {
    match error {
        LeaderboardError::NotFound => ApiError::NotFound,
        LeaderboardError::InvalidStoredData => ApiError::Internal,
        LeaderboardError::Database(error) => ApiError::Database(error),
    }
}
