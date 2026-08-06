use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("resource not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
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
        let (status, code, message) = match &self {
            Self::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, "validation_error", message.clone())
            }
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message.clone()),
            Self::Database(error) if is_constraint_violation(error) => (
                StatusCode::CONFLICT,
                "constraint_violation",
                constraint_message(error),
            ),
            Self::Database(error) => {
                tracing::error!(?error, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    self.to_string(),
                )
            }
        };
        (
            status,
            Json(ErrorEnvelope {
                error: ErrorBody { code, message },
            }),
        )
            .into_response()
    }
}

fn is_constraint_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation() || database.is_foreign_key_violation() || database.is_check_violation())
}

fn constraint_message(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Database(database) => database.message().to_owned(),
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
