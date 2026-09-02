use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::header::CACHE_CONTROL,
    response::IntoResponse,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    api::{
        auth::{AuthenticatedSession, MutationSession},
        authorization::map_authorization_error,
    },
    error::{ApiError, ApiResult},
    repositories::tournament_visibility::{self, FinalRoundVisibilityError},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/tournaments/{tournament_id}/final-round-visibility",
        get(get_visibility).patch(update_visibility),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateVisibility {
    back_nine_hidden: bool,
    expected_visibility_updated_at: DateTime<Utc>,
}

async fn get_visibility(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    let visibility = tournament_visibility::get_for_admin(
        &state.pool,
        authenticated.principal.user_id,
        tournament_id,
    )
    .await
    .map_err(map_error)?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(visibility)))
}

async fn update_visibility(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
    input: Result<Json<UpdateVisibility>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest(
            "request must contain only back_nine_hidden and expected_visibility_updated_at"
                .to_owned(),
        )
    })?;
    let result = tournament_visibility::update_authorized(
        &state.pool,
        authenticated.principal.session_id,
        tournament_id,
        input.back_nine_hidden,
        input.expected_visibility_updated_at,
    )
    .await
    .map_err(map_error)?;
    if result.changed {
        state.notify("visibility", tournament_id, tournament_id);
    }
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(result.visibility),
    ))
}

fn map_error(error: FinalRoundVisibilityError) -> ApiError {
    match error {
        FinalRoundVisibilityError::NotFound => ApiError::NotFound,
        FinalRoundVisibilityError::Stale => ApiError::DomainConflict {
            code: "final_round_visibility_stale",
            message: "final-round visibility changed; refresh and try again",
        },
        FinalRoundVisibilityError::Authorization(error) => map_authorization_error(error),
        FinalRoundVisibilityError::Database(error) => ApiError::Database(error),
    }
}
