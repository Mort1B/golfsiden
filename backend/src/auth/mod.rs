mod password;
mod session;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

pub use password::{hash_password, hash_password_bounded, verify_password};
pub use session::{
    SESSION_COOKIE_NAME, derive_csrf_token, generate_invitation_token, generate_session_token,
    hash_invitation_token, hash_session_token, verify_csrf_token, verify_derived_csrf,
    verify_invitation_token_hash,
};

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub cookie_secure: bool,
    pub session_ttl_hours: i64,
    pub cors_allowed_origin: Option<axum::http::HeaderValue>,
}

impl AuthConfig {
    pub fn local() -> Self {
        Self {
            cookie_secure: false,
            session_ttl_hours: 24,
            cors_allowed_origin: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Scorer,
    Player,
    Viewer,
}

#[derive(Debug, Clone, FromRow)]
pub struct SessionPrincipal {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub player_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionResponse {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub role: UserRole,
    pub player_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub csrf_token: String,
}

impl SessionPrincipal {
    pub fn response(&self, csrf_token: String) -> SessionResponse {
        SessionResponse {
            user_id: self.user_id,
            username: self.username.clone(),
            display_name: self.display_name.clone(),
            role: self.role,
            player_id: self.player_id,
            expires_at: self.expires_at,
            csrf_token,
        }
    }
}
