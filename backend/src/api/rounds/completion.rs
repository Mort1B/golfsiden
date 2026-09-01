use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::header::CACHE_CONTROL,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    api::{
        auth::{AuthenticatedSession, MutationSession},
        authorization::map_authorization_error,
    },
    domain::{
        models::Round,
        round_completion::{TransitionAction, TransitionBlocker},
    },
    error::{ApiError, ApiResult},
    repositories::round_completion::{self, RoundCompletionError},
};

pub async fn validation(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    let validation = round_completion::validation_for_member(
        &state.pool,
        authenticated.principal.user_id,
        round_id,
    )
    .await
    .map_err(map_authorization_error)?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(validation)))
}

pub async fn complete(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
) -> ApiResult<Json<Round>> {
    let round = round_completion::complete_authorized(
        &state.pool,
        authenticated.principal.session_id,
        round_id,
    )
    .await
    .map_err(map_error)?;
    state.notify("round", round.tournament_id, round_id);
    Ok(Json(round))
}

pub async fn lock(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
) -> ApiResult<Json<Round>> {
    let round = round_completion::lock_authorized(
        &state.pool,
        authenticated.principal.session_id,
        round_id,
    )
    .await
    .map_err(map_error)?;
    state.notify("round", round.tournament_id, round_id);
    Ok(Json(round))
}

fn map_error(error: RoundCompletionError) -> ApiError {
    match error {
        RoundCompletionError::NotFound => ApiError::NotFound,
        RoundCompletionError::Blocked { action, blocker } => {
            let (code, message) = conflict_details(action, blocker);
            ApiError::DomainConflict { code, message }
        }
        RoundCompletionError::Authorization(error) => map_authorization_error(error),
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
