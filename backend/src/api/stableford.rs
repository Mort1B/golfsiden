use crate::{
    AppState,
    api::auth::{AuthenticatedSession, MutationSession},
    domain::{
        scorecards::{ExpectedScore, ScoreAcknowledgement, ScoreOwner},
        stableford::card::{Card, Input, ReadEntry, read_projection},
    },
    error::{ApiError, ApiResult},
    repositories::{
        scorecards::ScorecardError,
        stableford::{self, SaveInput},
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
        .route("/api/rounds/{round_id}/stableford/settings", put(settings))
        .route(
            "/api/rounds/{round_id}/stableford/inputs/conditional",
            put(save),
        )
        .route(
            "/api/rounds/{round_id}/stableford/scorecards/{player_id}",
            get(read),
        )
        .route(
            "/api/rounds/{round_id}/stableford/scorecards/{player_id}/scoring",
            get(scoring),
        )
        .route(
            "/api/rounds/{round_id}/stableford/scorecards/{player_id}/confirm",
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
        body.map_err(|_| ApiError::BadRequest("invalid stableford input request".into()))?;
    input
        .input
        .domain()
        .map_err(|_| ApiError::BadRequest("gross_strokes must be between 1 and 20".into()))?;
    let PlayerOwner::Player { id } = input.owner;
    let result = stableford::save_conditional(
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
    Path((round, player)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Card<ReadEntry>>> {
    stableford::get(&state.pool, auth.principal.session_id, round, player, false)
        .await
        .map(read_projection)
        .map(Json)
        .map_err(map_error)
}
async fn scoring(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    Path((round, player)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Card>> {
    stableford::get(&state.pool, auth.principal.session_id, round, player, true)
        .await
        .map(Json)
        .map_err(map_error)
}
async fn confirm(
    State(state): State<Arc<AppState>>,
    MutationSession(auth): MutationSession,
    Path((round, player)): Path<(Uuid, Uuid)>,
    body: Result<Json<Empty>, JsonRejection>,
) -> ApiResult<Json<Card>> {
    let Json(_) =
        body.map_err(|_| ApiError::BadRequest("request body must be an empty object".into()))?;
    let result = stableford::confirm(&state.pool, auth.principal.session_id, round, player)
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsRequest {
    expected_round_updated_at: chrono::DateTime<chrono::Utc>,
    handicap_enabled: bool,
    handicap_allowance_percent: i16,
}
async fn settings(
    State(state): State<Arc<AppState>>,
    MutationSession(auth): MutationSession,
    Path(round_id): Path<Uuid>,
    body: Result<Json<SettingsRequest>, JsonRejection>,
) -> ApiResult<Json<crate::domain::models::Round>> {
    let Json(input) =
        body.map_err(|_| ApiError::BadRequest("invalid Stableford settings".into()))?;
    use crate::repositories::stableford::settings::{self, SettingsError};
    let round = settings::update(
        &state.pool,
        auth.principal.session_id,
        round_id,
        input.expected_round_updated_at,
        input.handicap_enabled,
        input.handicap_allowance_percent,
    )
    .await
    .map_err(|e| match e {
        SettingsError::Authorization(e) => crate::api::authorization::map_authorization_error(e),
        SettingsError::NotDraft => ApiError::DomainConflict {
            code: "round_not_draft",
            message: "Stableford settings require a draft round",
        },
        SettingsError::Stale => ApiError::DomainConflict {
            code: "round_configuration_stale",
            message: "round configuration has changed",
        },
        SettingsError::InvalidAllowance => {
            ApiError::BadRequest("allowance must be from 0 to 100".into())
        }
        SettingsError::Database(e) => ApiError::Database(e),
    })?;
    state.notify("round", round.tournament_id, round_id);
    Ok(Json(round))
}
