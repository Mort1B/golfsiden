mod management;
mod public;
use crate::repositories::tournament_authorization::AuthorizationError;
use chrono::{DateTime, Utc};
pub use management::{issue, revoke, status};
pub use public::{PublicRead, read};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Serialize, FromRow)]
pub struct GrantMetadata {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
#[derive(thiserror::Error, Debug)]
pub enum ShareError {
    #[error("result link unavailable")]
    Unavailable,
    #[error("overall results are not applicable")]
    OverallUnavailable,
    #[error("result link changed")]
    Stale,
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("invalid stored results")]
    InvalidResults,
}
const METADATA: &str = "id,created_at,expires_at,revoked_at";
