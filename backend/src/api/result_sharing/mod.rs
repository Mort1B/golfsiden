mod management;
mod public;
use crate::{
    AppState, api::authorization::map_authorization_error, error::ApiError,
    repositories::result_sharing::ShareError,
};
use axum::{
    Router,
    extract::{DefaultBodyLimit, rejection::JsonRejection},
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, REFERRER_POLICY},
    },
    middleware,
    response::Response,
    routing::{delete, get, post},
};
use std::sync::Arc;
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/tournaments/{tournament_id}/result-share",
            get(management::status).post(management::issue),
        )
        .route(
            "/api/tournaments/{tournament_id}/result-share/{grant_id}",
            delete(management::revoke),
        )
        .route("/api/public/results/{grant_id}", post(public::read))
        .layer(DefaultBodyLimit::max(2048))
        .layer(middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
                response
                    .headers_mut()
                    .insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
                response.headers_mut().insert(
                    "x-robots-tag",
                    HeaderValue::from_static("noindex, nofollow"),
                );
                response
            },
        ))
}
fn input_error(error: JsonRejection) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::PayloadTooLarge
    } else {
        ApiError::BadRequest("invalid result-sharing request".into())
    }
}
fn map_error(error: ShareError) -> ApiError {
    match error {
        ShareError::OverallUnavailable => ApiError::DomainConflict {
            code: "overall_not_applicable",
            message: "match-only tournaments have no public overall results",
        },
        ShareError::Unavailable => ApiError::DomainNotFound {
            code: "result_share_unavailable",
            message: "result link is unavailable",
        },
        ShareError::Stale => ApiError::DomainConflict {
            code: "result_share_stale",
            message: "result link changed; refresh and try again",
        },
        ShareError::Authorization(error) => map_authorization_error(error),
        // SQL errors may contain capability hash row details; never log them.
        ShareError::Database(_) | ShareError::InvalidResults => ApiError::Internal,
    }
}
