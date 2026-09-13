use crate::{
    AppState,
    api::auth::{AuthenticatedSession, MutationSession},
    domain::{
        four_ball_card::{Card, Input, ReadEntry, read_projection},
        scorecards::{ExpectedScore, ScoreAcknowledgement, ScoreOwner},
    },
    error::{ApiError, ApiResult},
    repositories::{
        four_ball::{self, SaveInput},
        scorecards::ScorecardError,
    },
};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderValue, header::CACHE_CONTROL},
    response::Response,
    routing::{get, post, put},
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    request_id: Uuid,
    hole_id: Uuid,
    owner: PlayerOwner,
    input: Input,
    expected_score: ExpectedScore,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum PlayerOwner {
    Player { id: Uuid },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/rounds/{round_id}/four-ball/inputs/conditional",
            put(save),
        )
        .route(
            "/api/rounds/{round_id}/four-ball/scorecards/{team_id}",
            get(read),
        )
        .route(
            "/api/rounds/{round_id}/four-ball/scorecards/{team_id}/scoring",
            get(scoring),
        )
        .route(
            "/api/rounds/{round_id}/four-ball/scorecards/{team_id}/confirm",
            post(confirm),
        )
        .layer(axum::extract::DefaultBodyLimit::max(2048))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
                response
            },
        ))
}
async fn save(
    State(state): State<Arc<AppState>>,
    MutationSession(auth): MutationSession,
    Path(round_id): Path<Uuid>,
    body: Result<Json<Request>, JsonRejection>,
) -> ApiResult<Json<ScoreAcknowledgement>> {
    let Json(input) =
        body.map_err(|_| ApiError::BadRequest("invalid four-ball input request".into()))?;
    input
        .input
        .domain()
        .map_err(|_| ApiError::BadRequest("gross_strokes must be between 1 and 20".into()))?;
    let PlayerOwner::Player { id } = input.owner;
    let result = four_ball::save_conditional(
        &state.pool,
        SaveInput {
            request_id: input.request_id,
            round_id,
            hole_id: input.hole_id,
            owner: ScoreOwner::Player { id },
            input: input.input,
            expected_score: input.expected_score,
            session_id: auth.principal.session_id,
        },
    )
    .await
    .map_err(map_error)?;
    if result.changed {
        state.notify("score", result.tournament_id, round_id);
    }
    Ok(Json(result.value))
}
async fn read(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((round, team)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Card<ReadEntry>>> {
    four_ball::get(&state.pool, auth.principal.session_id, round, team, false)
        .await
        .map(read_projection)
        .map(Json)
        .map_err(map_error)
}
async fn scoring(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((round, team)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Card>> {
    four_ball::get(&state.pool, auth.principal.session_id, round, team, true)
        .await
        .map(Json)
        .map_err(map_error)
}
async fn confirm(
    State(state): State<Arc<AppState>>,
    MutationSession(auth): MutationSession,
    Path((round, team)): Path<(Uuid, Uuid)>,
    body: Result<Json<Empty>, JsonRejection>,
) -> ApiResult<Json<Card>> {
    let Json(_) =
        body.map_err(|_| ApiError::BadRequest("request body must be an empty object".into()))?;
    let result = four_ball::confirm(&state.pool, auth.principal.session_id, round, team)
        .await
        .map_err(map_error)?;
    if result.changed {
        state.notify("score", result.tournament_id, round);
    }
    Ok(Json(result.value))
}
fn map_error(error: ScorecardError) -> ApiError {
    match error {
        ScorecardError::NotFound => ApiError::NotFound,
        ScorecardError::Forbidden => ApiError::Forbidden,
        ScorecardError::Unauthenticated => ApiError::Unauthenticated,
        ScorecardError::Conflict(c) => ApiError::DomainConflict {
            code: c.code(),
            message: c.message(),
        },
        _ => ApiError::Internal,
    }
}
