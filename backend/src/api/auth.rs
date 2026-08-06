use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{FromRequestParts, State, rejection::JsonRejection},
    http::{StatusCode, header::CACHE_CONTROL, request::Parts},
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::{
    AppState,
    auth::{
        SESSION_COOKIE_NAME, SessionPrincipal, derive_csrf_token, generate_session_token,
        hash_session_token, verify_derived_csrf, verify_password,
    },
    error::{ApiError, ApiResult},
    repositories::auth,
};

const DUMMY_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$Z29sZi1kdW1teS1zYWx0$g6+at6I1zni8XIFjKb4q79OHktQWrmrpswK13OU9sg0";
pub const CSRF_HEADER: &str = "x-csrf-token";

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/session", get(session))
        .route("/api/auth/logout", post(logout))
}

pub struct AuthenticatedSession {
    pub principal: SessionPrincipal,
    pub csrf_token: String,
}

pub struct MutationSession(pub AuthenticatedSession);

impl FromRequestParts<Arc<AppState>> for AuthenticatedSession {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let jar = match CookieJar::from_request_parts(parts, state).await {
            Ok(jar) => jar,
            Err(error) => match error {},
        };
        let token = jar
            .get(SESSION_COOKIE_NAME)
            .map(Cookie::value)
            .ok_or(ApiError::Unauthenticated)?;
        let hash = hash_session_token(token);
        let principal = auth::find_active_session(&state.pool, &hash)
            .await?
            .ok_or(ApiError::Unauthenticated)?;
        Ok(Self {
            principal,
            csrf_token: derive_csrf_token(token),
        })
    }
}

impl FromRequestParts<Arc<AppState>> for MutationSession {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let presented = parts
            .headers
            .get(CSRF_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
            .ok_or(ApiError::Forbidden)?;
        let session = AuthenticatedSession::from_request_parts(parts, state).await?;
        if !verify_derived_csrf(&session.csrf_token, &presented) {
            return Err(ApiError::Forbidden);
        }
        Ok(Self(session))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    email: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    input: Result<Json<LoginRequest>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let Json(input) = input
        .map_err(|_| ApiError::BadRequest("request must contain email and password".to_owned()))?;
    let user = auth::find_login_user(&state.pool, &input.email).await?;
    let encoded_hash = user
        .as_ref()
        .and_then(|user| user.password_hash.clone())
        .unwrap_or_else(|| DUMMY_PASSWORD_HASH.to_owned());
    if !verify_password(input.password, encoded_hash).await {
        return Err(ApiError::InvalidCredentials);
    }
    let user_id = user
        .map(|user| user.id)
        .ok_or(ApiError::InvalidCredentials)?;
    let token = generate_session_token().map_err(|_| ApiError::Internal)?;
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now() + Duration::hours(state.auth.session_ttl_hours);
    let principal = auth::create_session(&state.pool, user_id, &token_hash, expires_at).await?;
    let csrf_token = derive_csrf_token(&token);
    let cookie = session_cookie(&state, token, expires_at);
    Ok((
        jar.add(cookie),
        [(CACHE_CONTROL, "no-store")],
        Json(principal.response(csrf_token)),
    ))
}

async fn session(authenticated: AuthenticatedSession) -> impl IntoResponse {
    (
        [(CACHE_CONTROL, "no-store")],
        Json(authenticated.principal.response(authenticated.csrf_token)),
    )
}

async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    MutationSession(authenticated): MutationSession,
) -> ApiResult<impl IntoResponse> {
    auth::revoke_session(&state.pool, authenticated.principal.session_id).await?;
    let mut cookie = session_cookie(&state, String::new(), Utc::now());
    cookie.make_removal();
    Ok((
        jar.remove(cookie),
        [(CACHE_CONTROL, "no-store")],
        StatusCode::NO_CONTENT,
    ))
}

fn session_cookie(
    state: &AppState,
    token: String,
    expires_at: chrono::DateTime<Utc>,
) -> Cookie<'static> {
    let max_age = Duration::hours(state.auth.session_ttl_hours);
    Cookie::build((SESSION_COOKIE_NAME, token))
        .path("/api")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.auth.cookie_secure)
        .max_age(time::Duration::seconds(max_age.num_seconds()))
        .expires(time::OffsetDateTime::from_unix_timestamp(expires_at.timestamp()).ok())
        .build()
}
