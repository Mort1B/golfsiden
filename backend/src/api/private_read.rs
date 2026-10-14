use crate::{
    api::authorization::map_authorization_error, error::ApiError,
    repositories::session_read::SessionReadError,
};
use axum::{
    http::{HeaderValue, header::CACHE_CONTROL},
    response::Response,
};

pub(super) async fn response(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    response
}

pub(super) fn map_error(error: SessionReadError) -> ApiError {
    match error {
        SessionReadError::Authorization(error) => map_authorization_error(error),
        SessionReadError::Database(error) => ApiError::Database(error),
        SessionReadError::Unavailable => ApiError::ServiceUnavailable,
    }
}
