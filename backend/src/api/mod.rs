mod auth;
mod authorization;
mod leaderboards;
mod live;
mod players;
mod rounds;
mod scorecards;
mod teams;
mod tournaments;

use std::sync::Arc;

use axum::{
    Json, Router,
    http::{HeaderName, Method, header::CONTENT_TYPE},
    routing::get,
};
use serde_json::{Value, json};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    let cors_origin = state.auth.cors_allowed_origin.clone();
    let router = Router::new()
        .route("/api/health", get(health))
        .merge(auth::routes())
        .merge(players::routes())
        .merge(leaderboards::routes())
        .merge(tournaments::routes())
        .merge(rounds::routes())
        .merge(scorecards::routes())
        .merge(teams::routes())
        .route("/api/live", get(live::events))
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
