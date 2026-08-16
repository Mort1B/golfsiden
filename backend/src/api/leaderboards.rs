use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::header::CACHE_CONTROL,
    response::{IntoResponse, Response},
    routing::get,
};
use uuid::Uuid;

use crate::{
    AppState,
    api::{auth::AuthenticatedSession, authorization::map_authorization_error},
    domain::leaderboards::LeaderboardMetric,
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
    authenticated: AuthenticatedSession,
) -> ApiResult<Response> {
    round(
        &state,
        authenticated.principal.user_id,
        round_id,
        LeaderboardMetric::Gross,
    )
    .await
}

async fn round_net(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<Response> {
    round(
        &state,
        authenticated.principal.user_id,
        round_id,
        LeaderboardMetric::Net,
    )
    .await
}

async fn tournament_gross(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<Response> {
    tournament(
        &state,
        authenticated.principal.user_id,
        tournament_id,
        LeaderboardMetric::Gross,
    )
    .await
}

async fn tournament_net(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<Response> {
    tournament(
        &state,
        authenticated.principal.user_id,
        tournament_id,
        LeaderboardMetric::Net,
    )
    .await
}

async fn round(
    state: &AppState,
    user_id: Uuid,
    round_id: Uuid,
    metric: LeaderboardMetric,
) -> ApiResult<Response> {
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(
            leaderboards::round_for_member(&state.pool, user_id, round_id, metric)
                .await
                .map_err(map_error)?,
        ),
    )
        .into_response())
}

async fn tournament(
    state: &AppState,
    user_id: Uuid,
    tournament_id: Uuid,
    metric: LeaderboardMetric,
) -> ApiResult<Response> {
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(
            leaderboards::tournament_for_member(&state.pool, user_id, tournament_id, metric)
                .await
                .map_err(map_error)?,
        ),
    )
        .into_response())
}

fn map_error(error: LeaderboardError) -> ApiError {
    match error {
        LeaderboardError::NotFound => ApiError::NotFound,
        LeaderboardError::InvalidStoredData => ApiError::Internal,
        LeaderboardError::Authorization(error) => map_authorization_error(error),
        LeaderboardError::Database(error) => ApiError::Database(error),
    }
}
