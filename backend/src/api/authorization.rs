use crate::{error::ApiError, repositories::tournament_authorization::AuthorizationError};

pub fn map_authorization_error(error: AuthorizationError) -> ApiError {
    match error {
        AuthorizationError::NotFound => ApiError::NotFound,
        AuthorizationError::Unauthenticated => ApiError::Unauthenticated,
        AuthorizationError::Forbidden => ApiError::Forbidden,
        AuthorizationError::Database(error) => ApiError::Database(error),
    }
}
