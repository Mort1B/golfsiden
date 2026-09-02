mod auth;
mod authorization;
mod course_catalog;
mod course_provider;
mod invitations;
mod leaderboards;
mod live;
mod onboarding;
mod rounds;
mod scorecards;
mod teams;
mod tournament_visibility;
mod tournaments;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderName, Method, header::CONTENT_TYPE},
    routing::get,
};
use serde_json::{Value, json};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{AppState, error::ApiError, schema};

pub fn router(state: Arc<AppState>) -> Router {
    let cors_origin = state.auth.cors_allowed_origin.clone();
    let router = Router::new()
        .route("/api/health", get(health))
        .route("/api/ready", get(ready))
        .merge(auth::routes())
        .merge(course_catalog::routes())
        .merge(course_provider::routes())
        .merge(invitations::routes())
        .merge(onboarding::routes())
        .merge(leaderboards::routes())
        .merge(tournaments::routes())
        .merge(rounds::routes())
        .merge(scorecards::routes())
        .merge(teams::routes())
        .merge(tournament_visibility::routes())
        .route("/api/tournaments/{tournament_id}/live", get(live::events))
        .layer(TraceLayer::new_for_http());
    let router = if let Some(origin) = cors_origin {
        router.layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_credentials(true)
                .allow_headers([CONTENT_TYPE, HeaderName::from_static(auth::CSRF_HEADER)])
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                ]),
        )
    } else {
        router
    };
    router.with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn ready(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    schema::check_compatibility(&state.pool)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "readiness check failed");
            ApiError::ServiceUnavailable
        })?;
    Ok(Json(json!({ "status": "ready" })))
}
