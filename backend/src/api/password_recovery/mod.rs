mod admin;
mod public;
use crate::{AppState, error::ApiError, repositories::password_recovery::RecoveryError};
use axum::{
    Router,
    extract::{DefaultBodyLimit, rejection::JsonRejection},
    http::{
        StatusCode,
        header::{CACHE_CONTROL, REFERRER_POLICY},
    },
    middleware,
    response::Response,
    routing::post,
};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/tournaments/{tournament_id}/players/{player_id}/password-recovery",
            post(admin::issue),
        )
        .route(
            "/api/tournaments/{tournament_id}/players/{player_id}/password-recovery/revoke",
            post(admin::revoke),
        )
        .route(
            "/api/auth/password-recovery/{grant_id}/preview",
            post(public::preview),
        )
        .route(
            "/api/auth/password-recovery/{grant_id}/redeem",
            post(public::redeem),
        )
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("private, no-store"),
                );
                response.headers_mut().insert(
                    REFERRER_POLICY,
                    axum::http::HeaderValue::from_static("no-referrer"),
                );
                response
            },
        ))
}
fn input_error(error: JsonRejection) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::PayloadTooLarge
    } else {
        ApiError::BadRequest("invalid password recovery request".into())
    }
}
fn map_error(error: RecoveryError) -> ApiError {
    match error {
        RecoveryError::Invalid => ApiError::DomainConflict {
            code: "password_recovery_invalid",
            message: "recovery link is invalid or no longer available",
        },
        RecoveryError::Forbidden => ApiError::Forbidden,
        RecoveryError::Unauthenticated => ApiError::Unauthenticated,
        RecoveryError::IncorrectPassword => ApiError::DomainConflict {
            code: "current_password_incorrect",
            message: "current password is incorrect",
        },
        // Database errors can include values from sensitive rows. Do not pass
        // them to the generic database logger.
        RecoveryError::Database(_) => ApiError::Internal,
    }
}
