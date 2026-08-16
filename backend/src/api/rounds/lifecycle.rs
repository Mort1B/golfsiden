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
    domain::models::{OpenRoundResult, ReadinessIssueCode},
    error::{ApiError, ApiResult},
    repositories::round_lifecycle::{self, OpenRoundError},
};

pub async fn pairing_validation(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    let validation = round_lifecycle::pairing_validation_for_member(
        &state.pool,
        authenticated.principal.user_id,
        round_id,
    )
    .await
    .map_err(map_authorization_error)?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(validation)))
}

pub async fn open(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
) -> ApiResult<Json<OpenRoundResult>> {
    let result =
        round_lifecycle::open_authorized(&state.pool, authenticated.principal.session_id, round_id)
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
        OpenRoundError::Authorization(error) => map_authorization_error(error),
        OpenRoundError::Database(error) => ApiError::Database(error),
    }
}
