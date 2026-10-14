use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderValue, header::CACHE_CONTROL},
    middleware,
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
        .layer(middleware::map_response(private_response))
}

async fn private_response(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    response
}

async fn round_gross(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<Response> {
    round(
        &state,
        authenticated.principal.session_id,
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
        authenticated.principal.session_id,
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
        authenticated.principal.session_id,
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
        authenticated.principal.session_id,
        tournament_id,
        LeaderboardMetric::Net,
    )
    .await
}

async fn round(
    state: &AppState,
    session_id: Uuid,
    round_id: Uuid,
    metric: LeaderboardMetric,
) -> ApiResult<Response> {
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(
            leaderboards::round_for_session(&state.pool, session_id, round_id, metric)
                .await
                .map_err(map_error)?,
        ),
    )
        .into_response())
}

async fn tournament(
    state: &AppState,
    session_id: Uuid,
    tournament_id: Uuid,
    metric: LeaderboardMetric,
) -> ApiResult<Response> {
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(
            leaderboards::tournament_response_for_session(
                &state.pool,
                session_id,
                tournament_id,
                metric,
            )
            .await
            .map_err(map_error)?,
        ),
    )
        .into_response())
}

fn map_error(error: LeaderboardError) -> ApiError {
    match error {
        LeaderboardError::NotFound => ApiError::NotFound,
        LeaderboardError::OverallUnavailable => ApiError::DomainConflict {
            code: "overall_not_applicable",
            message: "match play has separate match results",
        },
        LeaderboardError::InvalidStoredData => ApiError::Internal,
        LeaderboardError::ReadUnavailable => ApiError::ServiceUnavailable,
        LeaderboardError::Authorization(error) => map_authorization_error(error),
        LeaderboardError::Database(error) => ApiError::Database(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn exhausted_read_retries_return_a_private_generic_unavailable_response() {
        let response =
            private_response(map_error(LeaderboardError::ReadUnavailable).into_response()).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()[CACHE_CONTROL], "private, no-store");
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            serde_json::json!({"error":{"code":"service_unavailable","message":"service is not ready"}})
        );
    }
}
