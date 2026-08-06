use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    AppState,
    domain::models::{OpenRoundResult, PairingValidation, ReadinessIssueCode},
    error::{ApiError, ApiResult},
    repositories::round_lifecycle::{self, OpenRoundError},
};

pub async fn pairing_validation(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<PairingValidation>> {
    let validation = round_lifecycle::pairing_validation(&state.pool, round_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(validation))
}

pub async fn open(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<OpenRoundResult>> {
    let result = round_lifecycle::open(&state.pool, round_id)
        .await
        .map_err(map_open_error)?;
    state.notify("round", round_id);
    Ok(Json(result))
}

fn map_open_error(error: OpenRoundError) -> ApiError {
    match error {
        OpenRoundError::NotFound => ApiError::NotFound,
        OpenRoundError::NotReady(validation)
            if validation
                .issues
                .iter()
                .any(|issue| issue.code == ReadinessIssueCode::RoundNotDraft) =>
        {
            ApiError::Conflict("round must be draft".to_owned())
        }
        OpenRoundError::NotReady(_) => ApiError::Conflict("round is not ready to open".to_owned()),
        OpenRoundError::Database(error) => ApiError::Database(error),
    }
}
