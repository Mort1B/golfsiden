mod admin;
mod public;
use crate::{AppState, error::ApiError, repositories::player_claims::ClaimError};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/tournaments/{tournament_id}/player-accounts",
            get(admin::list),
        )
        .route(
            "/api/tournaments/{tournament_id}/players",
            post(admin::create),
        )
        .route(
            "/api/tournaments/{tournament_id}/players/{player_id}/claim",
            post(admin::reissue).delete(admin::revoke),
        )
        .route(
            "/api/tournaments/{tournament_id}/players/{player_id}",
            delete(admin::withdraw),
        )
        .route(
            "/api/player-claims/{claim_id}/preview",
            post(public::preview),
        )
        .route(
            "/api/player-claims/{claim_id}/register",
            post(public::register),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::map_response(no_store))
}
async fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
#[derive(Debug)]
pub(super) enum Error {
    Api(ApiError),
    Claim(ClaimError),
}
impl From<ApiError> for Error {
    fn from(e: ApiError) -> Self {
        Self::Api(e)
    }
}
impl From<ClaimError> for Error {
    fn from(e: ClaimError) -> Self {
        Self::Claim(e)
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let error = match self {
            Self::Api(e) => return e.into_response(),
            Self::Claim(e) => e,
        };
        let (status, code, message) = match error {
            ClaimError::Invalid => (StatusCode::NOT_FOUND, "claim_invalid", "claim is invalid"),
            ClaimError::Unavailable => (
                StatusCode::GONE,
                "claim_unavailable",
                "claim has expired, been revoked, or already been used",
            ),
            ClaimError::HasAccount => (
                StatusCode::CONFLICT,
                "player_has_account",
                "player already has an account",
            ),
            ClaimError::Withdrawn => (
                StatusCode::CONFLICT,
                "player_withdrawn",
                "player is withdrawn or inactive",
            ),
            ClaimError::Closed => (
                StatusCode::CONFLICT,
                "tournament_not_joinable",
                "tournament is closed",
            ),
            ClaimError::Admin => (
                StatusCode::CONFLICT,
                "player_is_admin",
                "administrators cannot be removed",
            ),
            ClaimError::DraftAssignment => (
                StatusCode::CONFLICT,
                "player_assigned_draft",
                "remove the player's draft team, flight and match assignments first",
            ),
            ClaimError::LiveRound => (
                StatusCode::CONFLICT,
                "player_round_in_progress",
                "lock the player's open or completed rounds first",
            ),
            ClaimError::Busy => (
                StatusCode::CONFLICT,
                "player_participation_changed",
                "player participation changed; refresh and retry",
            ),
            ClaimError::Username => (
                StatusCode::CONFLICT,
                "username_already_registered",
                "an account with this username already exists",
            ),
            ClaimError::Authorization(e) => {
                return super::authorization::map_authorization_error(e).into_response();
            }
            ClaimError::Database(e) => return ApiError::Database(e).into_response(),
        };
        (
            status,
            axum::Json(serde_json::json!({"error":{"code":code,"message":message}})),
        )
            .into_response()
    }
}
