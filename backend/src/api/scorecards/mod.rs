use std::sync::Arc;

use axum::{
    Json, Router,
    extract::rejection::JsonRejection,
    extract::{Path, State},
    routing::{get, post, put},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    domain::{
        scorecards::{ScoreEntry, ScoreOwner, ScorecardSummary},
        scoring::ScoringError,
    },
    error::{ApiError, ApiResult},
    repositories::scorecards::{self, SaveScore, ScorecardError},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/rounds/{round_id}/scores", put(save))
        .route(
            "/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}",
            get(get_one),
        )
        .route(
            "/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}/confirm",
            post(confirm),
        )
}

#[derive(Deserialize)]
struct SaveScoreRequest {
    hole_id: Uuid,
    owner: ScoreOwner,
    gross_strokes: i16,
    submitted_by: Uuid,
}

#[derive(Deserialize)]
struct ConfirmRequest {
    confirmed_by: Uuid,
}

async fn save(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    input: Result<Json<SaveScoreRequest>, JsonRejection>,
) -> ApiResult<Json<ScoreEntry>> {
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest("request must contain a valid tagged score owner".to_owned())
    })?;
    if !(1..=20).contains(&input.gross_strokes) {
        return Err(ApiError::BadRequest(
            "gross_strokes must be between 1 and 20".to_owned(),
        ));
    }
    let result = scorecards::save(
        &state.pool,
        SaveScore {
            round_id,
            hole_id: input.hole_id,
            owner: input.owner,
            gross_strokes: input.gross_strokes,
            submitted_by: input.submitted_by,
        },
    )
    .await
    .map_err(map_error)?;
    if result.changed {
        state.notify("score", round_id);
    }
    Ok(Json(result.value))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    Path((round_id, owner_type, owner_id)): Path<(Uuid, String, Uuid)>,
) -> ApiResult<Json<ScorecardSummary>> {
    let owner = parse_owner(&owner_type, owner_id)?;
    Ok(Json(
        scorecards::get(&state.pool, round_id, owner)
            .await
            .map_err(map_error)?,
    ))
}

async fn confirm(
    State(state): State<Arc<AppState>>,
    Path((round_id, owner_type, owner_id)): Path<(Uuid, String, Uuid)>,
    input: Result<Json<ConfirmRequest>, JsonRejection>,
) -> ApiResult<Json<ScorecardSummary>> {
    let Json(input) = input
        .map_err(|_| ApiError::BadRequest("confirmed_by must be a valid user ID".to_owned()))?;
    let owner = parse_owner(&owner_type, owner_id)?;
    let result = scorecards::confirm(&state.pool, round_id, owner, input.confirmed_by)
        .await
        .map_err(map_error)?;
    if result.changed {
        state.notify("score", round_id);
    }
    Ok(Json(result.value))
}

fn parse_owner(owner_type: &str, owner_id: Uuid) -> ApiResult<ScoreOwner> {
    match owner_type {
        "player" => Ok(ScoreOwner::Player { id: owner_id }),
        "team" => Ok(ScoreOwner::Team { id: owner_id }),
        _ => Err(ApiError::BadRequest(
            "owner_type must be player or team".to_owned(),
        )),
    }
}

fn map_error(error: ScorecardError) -> ApiError {
    match error {
        ScorecardError::NotFound => ApiError::NotFound,
        ScorecardError::Conflict(conflict) => ApiError::DomainConflict {
            code: conflict.code(),
            message: conflict.message(),
        },
        ScorecardError::Scoring(ScoringError::InvalidTeamSize) => ApiError::DomainConflict {
            code: "score_owner_not_eligible",
            message: "score owner is not eligible for this round",
        },
        ScorecardError::Database(error) => ApiError::Database(error),
        ScorecardError::InvalidStoredData | ScorecardError::Scoring(_) => ApiError::Internal,
    }
}
