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
    api::{auth::AuthenticatedSession, authorization::map_authorization_error},
    error::ApiResult,
    repositories::teams,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/rounds/{round_id}/teams", get(list))
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    let teams = teams::list_for_member(&state.pool, authenticated.principal.user_id, round_id)
        .await
        .map_err(map_authorization_error)?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(teams)))
}
