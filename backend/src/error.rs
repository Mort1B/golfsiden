use axum::{
    Json,
    http::{
        HeaderValue, StatusCode,
        header::{CACHE_CONTROL, RETRY_AFTER},
    },
    response::IntoResponse,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("request body is too large")]
    PayloadTooLarge,
    #[error("resource not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("{message}")]
    DomainConflict {
        code: &'static str,
        message: &'static str,
    },
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("authentication required")]
    Unauthenticated,
    #[error("request is not permitted")]
    Forbidden,
    #[error("too many requests; try again later")]
    RateLimited { retry_after_seconds: u64 },
    #[error("service is not ready")]
    ServiceUnavailable,
    #[error("internal server error")]
    Internal,
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message, retry_after) = match &self {
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                "validation_error",
                message.clone(),
                None,
            ),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                self.to_string(),
                None,
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string(), None),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message.clone(), None),
            Self::DomainConflict { code, message } => {
                (StatusCode::CONFLICT, *code, (*message).to_owned(), None)
            }
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                self.to_string(),
                None,
            ),
            Self::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                self.to_string(),
                None,
            ),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", self.to_string(), None),
            Self::RateLimited {
                retry_after_seconds,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                self.to_string(),
                Some(*retry_after_seconds),
            ),
            Self::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                self.to_string(),
                None,
            ),
            Self::Internal => {
                tracing::error!("internal application error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    self.to_string(),
                    None,
                )
            }
            Self::Database(error) if is_constraint_violation(error) => (
                StatusCode::CONFLICT,
                "constraint_violation",
                constraint_message(error),
                None,
            ),
            Self::Database(error) => {
                tracing::error!(?error, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    self.to_string(),
                    None,
                )
            }
        };
        let mut response = (
            status,
            Json(ErrorEnvelope {
                error: ErrorBody { code, message },
            }),
        )
            .into_response();
        if let Some(seconds) = retry_after
            && let Ok(value) = HeaderValue::from_str(&seconds.to_string())
        {
            response.headers_mut().insert(RETRY_AFTER, value);
        }
        if matches!(
            status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS
        ) {
            response
                .headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        }
        response
    }
}

fn is_constraint_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation() || database.is_foreign_key_violation() || database.is_check_violation())
}

fn constraint_message(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Database(database) => match database.constraint() {
            Some("round_configuration_frozen") => {
                "round scoring configuration is frozen after draft".to_owned()
            }
            Some("round_pairing_frozen") => "round pairings are frozen after draft".to_owned(),
            Some("tee_configuration_frozen") => {
                "tee configuration is frozen after round opening".to_owned()
            }
            Some("hole_configuration_frozen") => {
                "hole configuration is frozen after round opening".to_owned()
            }
            Some("round_snapshot_capture_frozen") => {
                "round handicap snapshots can only be captured by the opening workflow".to_owned()
            }
            Some("round_snapshot_immutable") => "round handicap snapshots are immutable".to_owned(),
            Some("round_status_transition_invalid") => {
                "round status transition is not allowed".to_owned()
            }
            Some("round_opening_context_required") => {
                "rounds must be opened through the lifecycle workflow".to_owned()
            }
            Some("round_opening_snapshots_incomplete") => {
                "round opening requires one snapshot per active entrant".to_owned()
            }
            Some("round_completion_context_required") => {
                "rounds must be completed through the completion workflow".to_owned()
            }
            Some("round_lock_context_required") => {
                "rounds must be locked through the locking workflow".to_owned()
            }
            Some("round_scorecards_not_ready") => {
                "round requires complete confirmed scorecards".to_owned()
            }
            Some("score_mutation_context_required") => {
                "scores must be changed through the score workflow".to_owned()
            }
            Some("score_delete_forbidden") => {
                "scores cannot be deleted while their round exists".to_owned()
            }
            Some("score_round_not_editable") => {
                "scores require an open or completed round".to_owned()
            }
            Some("score_round_lock_required") => {
                "score mutation could not acquire the round lock".to_owned()
            }
            Some("score_identity_immutable") => "score identity is immutable".to_owned(),
            Some("score_unchanged_submitter") => {
                "unchanged scores cannot replace their submitter".to_owned()
            }
            Some("score_hole_not_in_round") => "hole does not belong to the round tee".to_owned(),
            Some("score_owner_format_mismatch") => {
                "score owner does not match the round format".to_owned()
            }
            Some("score_owner_ineligible") => {
                "score owner is not eligible for this round".to_owned()
            }
            Some("score_confirmation_context_required") => {
                "scorecard confirmation must use the score workflow".to_owned()
            }
            Some("score_confirmation_immutable") => {
                "scorecard confirmations are immutable".to_owned()
            }
            Some("scorecard_incomplete") => "scorecard is incomplete".to_owned(),
            Some("score_audit_context_required") => {
                "score audits must be created by the score workflow".to_owned()
            }
            Some("score_audit_immutable") => "score audit history is append-only".to_owned(),
            _ => "request violates a data constraint".to_owned(),
        },
        _ => "request violates a data constraint".to_owned(),
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

pub fn require_non_empty(value: &str, field: &str) -> ApiResult<()> {
    if value.trim().is_empty() {
        return Err(ApiError::BadRequest(format!("{field} must not be empty")));
    }
    Ok(())
}
