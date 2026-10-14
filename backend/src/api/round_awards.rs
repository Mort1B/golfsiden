use crate::{
    AppState,
    api::{
        auth::{AuthenticatedSession, MutationSession},
        authorization::map_authorization_error,
    },
    error::{ApiError, ApiResult},
    repositories::round_awards::{self, Command, Error, Receipt, View},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::{HeaderValue, header::CACHE_CONTROL},
    middleware,
    response::Response,
    routing::get,
};
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/rounds/{round_id}/awards", get(read).put(write))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::map_response(no_store))
}
async fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    response
}
async fn read(
    State(state): State<Arc<AppState>>,
    Path(round): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<Json<View>> {
    round_awards::read(&state.pool, authenticated.principal.session_id, round)
        .await
        .map(Json)
        .map_err(map_error)
}
async fn write(
    State(state): State<Arc<AppState>>,
    Path(round): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
    body: Result<Json<Command>, JsonRejection>,
) -> ApiResult<Json<Receipt>> {
    let Json(command) = body.map_err(|error| {
        if error.status() == axum::http::StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::PayloadTooLarge
        } else {
            ApiError::BadRequest("invalid round award command".into())
        }
    })?;
    let (tournament, receipt, changed) = round_awards::write(
        &state.pool,
        authenticated.principal.session_id,
        round,
        &command,
    )
    .await
    .map_err(map_error)?;
    if changed {
        state.notify("round", tournament, round);
    }
    Ok(Json(receipt))
}
fn map_error(error: Error) -> ApiError {
    let (code, message) = match error {
        Error::Authorization(e) => return map_authorization_error(e),
        Error::Database(e) => return ApiError::Database(e),
        Error::Invalid => {
            return ApiError::BadRequest("invalid round award command or participant".into());
        }
        Error::Revision => (
            "round_award_revision_conflict",
            "award revision changed; this request was not accepted",
        ),
        Error::Request => (
            "round_award_request_conflict",
            "request identity was used for another command",
        ),
        Error::State => (
            "round_award_not_completed",
            "round must be completed or locked",
        ),
        Error::Format => (
            "round_award_unsupported_format",
            "match play has no net stroke award",
        ),
        Error::Correction => (
            "round_award_correction_required",
            "explicit correction acknowledgement is required",
        ),
    };
    ApiError::DomainConflict { code, message }
}
