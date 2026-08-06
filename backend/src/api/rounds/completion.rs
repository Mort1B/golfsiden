use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{
        models::Round,
        round_completion::{RoundCompletionValidation, TransitionAction, TransitionBlocker},
    },
    error::{ApiError, ApiResult},
    repositories::round_completion::{self, RoundCompletionError},
};

pub async fn validation(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<RoundCompletionValidation>> {
    let validation = round_completion::validation(&state.pool, round_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(validation))
}

pub async fn complete(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<Round>> {
    let round = round_completion::complete(&state.pool, round_id)
        .await
        .map_err(map_error)?;
    state.notify("round", round_id);
    Ok(Json(round))
}

pub async fn lock(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<Round>> {
    let round = round_completion::lock(&state.pool, round_id)
        .await
        .map_err(map_error)?;
    state.notify("round", round_id);
    Ok(Json(round))
}

fn map_error(error: RoundCompletionError) -> ApiError {
    match error {
        RoundCompletionError::NotFound => ApiError::NotFound,
        RoundCompletionError::Blocked { action, blocker } => {
            let (code, message) = conflict_details(action, blocker);
            ApiError::DomainConflict { code, message }
        }
        RoundCompletionError::Database(error) => ApiError::Database(error),
    }
}

fn conflict_details(
    action: TransitionAction,
    blocker: TransitionBlocker,
) -> (&'static str, &'static str) {
    match blocker {
        TransitionBlocker::InvalidSourceState => match action {
            TransitionAction::Complete => ("round_not_open", "round must be open to complete"),
            TransitionAction::Lock => ("round_not_completed", "round must be completed to lock"),
        },
        TransitionBlocker::NoRequiredOwners => (
            "round_has_no_scorecards",
            "round has no required scorecards",
        ),
        TransitionBlocker::IncompleteScorecards => (
            "round_scorecards_incomplete",
            "one or more scorecards are incomplete",
        ),
        TransitionBlocker::UnconfirmedScorecards => (
            "round_scorecards_unconfirmed",
            "one or more scorecards are unconfirmed",
        ),
    }
}
