mod admin;
mod public;

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;
use uuid::Uuid;

use crate::{AppState, error::ApiError, repositories::invitations::InvitationError};

const MAX_INVITATION_BODY_BYTES: usize = 16 * 1024;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/invitations/{invitation_id}/preview",
            post(public::preview),
        )
        .route(
            "/api/invitations/{invitation_id}/register",
            post(public::register),
        )
        .route(
            "/api/invitations/{invitation_id}/accept",
            post(public::accept),
        )
        .route("/api/invitations/preview", post(public::missing_id))
        .route("/api/invitations/register", post(public::missing_id))
        .route("/api/invitations/accept", post(public::missing_id))
        .route(
            "/api/tournaments/{tournament_id}/invitations",
            get(admin::list).post(admin::issue),
        )
        .route(
            "/api/tournaments/{tournament_id}/invitations/{invitation_id}/rotate",
            post(admin::rotate),
        )
        .route(
            "/api/tournaments/{tournament_id}/invitations/{invitation_id}",
            axum::routing::delete(admin::revoke),
        )
        .layer(DefaultBodyLimit::max(MAX_INVITATION_BODY_BYTES))
        .layer(middleware::map_response(no_store))
}

pub(super) fn parse_public_id(value: &str) -> Result<Uuid, InvitationApiError> {
    Uuid::parse_str(value).map_err(|_| InvitationError::Invalid.into())
}

pub(super) fn parse_management_id(value: &str) -> Result<Uuid, InvitationApiError> {
    Uuid::parse_str(value).map_err(|_| InvitationError::NotFound.into())
}

#[derive(Debug)]
pub(super) enum InvitationApiError {
    Api(ApiError),
    Invitation(InvitationError),
}

impl From<ApiError> for InvitationApiError {
    fn from(value: ApiError) -> Self {
        Self::Api(value)
    }
}

impl From<InvitationError> for InvitationApiError {
    fn from(value: InvitationError) -> Self {
        Self::Invitation(value)
    }
}

impl IntoResponse for InvitationApiError {
    fn into_response(self) -> Response {
        let error = match self {
            Self::Api(error) => return error.into_response(),
            Self::Invitation(error) => error,
        };
        let (status, code, message) = match error {
            InvitationError::Invalid => (
                StatusCode::NOT_FOUND,
                "invitation_invalid",
                "invitation is invalid",
            ),
            InvitationError::Expired => (
                StatusCode::GONE,
                "invitation_expired",
                "invitation has expired",
            ),
            InvitationError::Revoked => (
                StatusCode::GONE,
                "invitation_revoked",
                "invitation has been revoked",
            ),
            InvitationError::Exhausted => (
                StatusCode::CONFLICT,
                "invitation_exhausted",
                "invitation has no remaining uses",
            ),
            InvitationError::TournamentNotJoinable => (
                StatusCode::CONFLICT,
                "tournament_not_joinable",
                "tournament is not accepting players",
            ),
            InvitationError::DuplicateUsername => (
                StatusCode::CONFLICT,
                "username_already_registered",
                "an account with this username already exists",
            ),
            InvitationError::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "authentication required",
            ),
            InvitationError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "request is not permitted",
            ),
            InvitationError::UnlinkedPlayer => (
                StatusCode::CONFLICT,
                "account_player_required",
                "the account is not linked to a player",
            ),
            InvitationError::InactivePlayer => (
                StatusCode::CONFLICT,
                "player_inactive",
                "the linked player is inactive",
            ),
            InvitationError::WithdrawnPlayer => (
                StatusCode::CONFLICT,
                "player_withdrawn",
                "the tournament entrant is withdrawn",
            ),
            InvitationError::RotationConflict => (
                StatusCode::CONFLICT,
                "invitation_rotation_conflict",
                "only an active invitation can be rotated",
            ),
            InvitationError::NotFound => {
                return ApiError::NotFound.into_response();
            }
            InvitationError::Database(error) => {
                return ApiError::Database(error).into_response();
            }
        };
        (
            status,
            axum::Json(json!({"error": {"code": code, "message": message}})),
        )
            .into_response()
    }
}

async fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
