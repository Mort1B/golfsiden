use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::header::CACHE_CONTROL,
    response::IntoResponse,
    routing::get,
};
use uuid::Uuid;

use crate::{
    AppState,
    api::{
        auth::AuthenticatedSession,
        private_read::{map_error as map_read_error, response as private_response},
    },
    error::ApiResult,
    repositories::teams,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/rounds/{round_id}/teams",
        get(list).layer(axum::middleware::map_response(private_response)),
    )
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    let teams = teams::list_for_session(&state.pool, authenticated.principal.session_id, round_id)
        .await
        .map_err(map_read_error)?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(teams)))
}
