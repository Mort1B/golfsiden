use crate::{
    AppState,
    api::auth::{AuthenticatedSession, MutationSession},
    domain::match_play::commands::{Acknowledgement, Request},
    error::{ApiError, ApiResult},
    repositories::match_play::{
        self, Error,
        reads::{Card, Listing, Table},
        setup::{Assignments, Settings},
    },
};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderValue, header::CACHE_CONTROL},
    response::Response,
    routing::{get, post, put},
};
use std::sync::Arc;
use uuid::Uuid;
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/rounds/{round}/match-play/settings", put(settings))
        .route(
            "/api/rounds/{round}/match-play/matches",
            get(list).put(assign),
        )
        .route("/api/rounds/{round}/match-play/matches/{id}", get(read))
        .route(
            "/api/rounds/{round}/match-play/matches/{id}/scoring",
            get(scoring),
        )
        .route(
            "/api/rounds/{round}/match-play/matches/{id}/commands",
            post(command),
        )
        .route("/api/rounds/{round}/match-play/completion", get(completion))
        .route("/api/tournaments/{tournament}/match-table", get(table))
        .layer(axum::extract::DefaultBodyLimit::max(32768))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
                response
            },
        ))
}
async fn command(
    State(state): State<Arc<AppState>>,
    MutationSession(auth): MutationSession,
    Path((round, id)): Path<(Uuid, Uuid)>,
    body: Result<Json<Request>, JsonRejection>,
) -> ApiResult<Json<Acknowledgement>> {
    let Json(request) = body.map_err(|_| ApiError::BadRequest("invalid match command".into()))?;
    let result = match_play::execute(&state.pool, auth.principal.session_id, round, id, request)
        .await
        .map_err(map_error)?;
    if result.changed {
        state.notify("match", result.tournament_id, round);
    }
    Ok(Json(result.value))
}
async fn read(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((round, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Card>> {
    match_play::reads::get(&state.pool, auth.principal.session_id, round, id, false)
        .await
        .map(Json)
        .map_err(map_error)
}
async fn scoring(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((round, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Card>> {
    match_play::reads::get(&state.pool, auth.principal.session_id, round, id, true)
        .await
        .map(Json)
        .map_err(map_error)
}
async fn list(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(round): Path<Uuid>,
) -> ApiResult<Json<Listing>> {
    match_play::reads::list(&state.pool, auth.principal.session_id, round)
        .await
        .map(Json)
        .map_err(map_error)
}
async fn assign(
    State(state): State<Arc<AppState>>,
    MutationSession(auth): MutationSession,
    Path(round): Path<Uuid>,
    body: Result<Json<Assignments>, JsonRejection>,
) -> ApiResult<Json<Listing>> {
    let Json(input) = body.map_err(|_| ApiError::BadRequest("invalid match assignments".into()))?;
    let tournament =
        match_play::setup::replace(&state.pool, auth.principal.session_id, round, input)
            .await
            .map_err(map_error)?;
    state.notify("round", tournament, round);
    match_play::reads::list(&state.pool, auth.principal.session_id, round)
        .await
        .map(Json)
        .map_err(map_error)
}
async fn settings(
    State(state): State<Arc<AppState>>,
    MutationSession(auth): MutationSession,
    Path(round): Path<Uuid>,
    body: Result<Json<Settings>, JsonRejection>,
) -> ApiResult<Json<crate::domain::models::Round>> {
    let Json(input) = body.map_err(|_| ApiError::BadRequest("invalid match settings".into()))?;
    let r = match_play::setup::settings(&state.pool, auth.principal.session_id, round, input)
        .await
        .map_err(map_error)?;
    state.notify("round", r.tournament_id, round);
    Ok(Json(r))
}
async fn completion(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(round): Path<Uuid>,
) -> ApiResult<Json<match_play::completion::Completion>> {
    match_play::completion::get(&state.pool, auth.principal.session_id, round)
        .await
        .map(Json)
        .map_err(map_error)
}
async fn table(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path(tournament): Path<Uuid>,
) -> ApiResult<Json<Table>> {
    match_play::reads::table(&state.pool, auth.principal.session_id, tournament)
        .await
        .map(Json)
        .map_err(map_error)
}
fn map_error(e: Error) -> ApiError {
    match e {
        Error::NotFound => ApiError::NotFound,
        Error::Unauthenticated => ApiError::Unauthenticated,
        Error::Forbidden => ApiError::Forbidden,
        Error::Invalid(message) => ApiError::BadRequest(message.into()),
        Error::Conflict(code) => ApiError::DomainConflict {
            code,
            message: code,
        },
        Error::Database(e) => ApiError::Database(e),
    }
}
