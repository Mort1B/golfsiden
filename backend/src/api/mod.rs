mod leaderboards;
mod live;
mod players;
mod rounds;
mod scorecards;
mod teams;
mod tournaments;

use std::sync::Arc;

use axum::{Json, Router, http::Method, routing::get};
use serde_json::{Value, json};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .merge(players::routes())
        .merge(leaderboards::routes())
        .merge(tournaments::routes())
        .merge(rounds::routes())
        .merge(scorecards::routes())
        .merge(teams::routes())
        .route("/api/live", get(live::events))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                ]),
        )
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
