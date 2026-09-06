use std::sync::Arc;

use super::auth::{AuthenticatedSession, MutationSession, session_cookie};
use crate::{
    AppState,
    auth::{hash_password_bounded, verify_password_bounded},
    domain::accounts::{normalize_and_validate_username, validate_password_length},
    error::{ApiError, ApiResult},
    rate_limit::RateLimitRoute,
    repositories::profile::{self, CredentialChange, Profile, ProfileError},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header::CACHE_CONTROL},
    middleware,
    response::Response,
    routing::{get, post},
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::Deserialize;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/profile", get(get_profile).put(update_details))
        .route("/api/me/profile/username", post(update_username))
        .route("/api/me/profile/password", post(update_password))
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("private, no-store"),
                );
                response
            },
        ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DetailsRequest {
    version: i64,
    player_updated_at: Option<DateTime<Utc>>,
    display_name: String,
    handicap: Option<f64>,
    reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsernameRequest {
    version: i64,
    username: String,
    current_password: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordRequest {
    version: i64,
    new_password: String,
    current_password: String,
}

fn input_error(error: JsonRejection) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::PayloadTooLarge
    } else {
        ApiError::BadRequest("invalid profile request".into())
    }
}
fn map_error(error: ProfileError) -> ApiError {
    match error {
        ProfileError::Unauthenticated => ApiError::Unauthenticated,
        ProfileError::IncorrectPassword => ApiError::DomainConflict {
            code: "current_password_incorrect",
            message: "current password is incorrect",
        },
        ProfileError::Stale => ApiError::DomainConflict {
            code: "profile_stale",
            message: "profile changed; reload before editing",
        },
        ProfileError::Inactive => ApiError::DomainConflict {
            code: "profile_inactive",
            message: "inactive player handicap cannot be changed",
        },
        ProfileError::MissingHandicap | ProfileError::MissingReason => {
            ApiError::BadRequest(error.to_string())
        }
        ProfileError::Database(sqlx::Error::Database(ref db))
            if db.constraint() == Some("users_username_normalized_idx") =>
        {
            ApiError::DomainConflict {
                code: "username_unavailable",
                message: "username is unavailable",
            }
        }
        ProfileError::Database(error) => ApiError::Database(error),
    }
}

async fn get_profile(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> ApiResult<Json<Profile>> {
    Ok(Json(
        profile::get(&state.pool, session.principal.session_id)
            .await
            .map_err(map_error)?,
    ))
}
async fn update_details(
    State(state): State<Arc<AppState>>,
    MutationSession(session): MutationSession,
    input: Result<Json<DetailsRequest>, JsonRejection>,
) -> ApiResult<Json<Profile>> {
    let Json(input) = input.map_err(input_error)?;
    let display_name = input.display_name.trim().to_owned();
    let reason = input.reason.trim().to_owned();
    if display_name.is_empty()
        || display_name.chars().count() > 100
        || reason.chars().count() > 500
        || input.version < 0
    {
        return Err(ApiError::BadRequest(
            "name must contain 1 to 100 characters; reason at most 500 characters".into(),
        ));
    }
    if input.handicap.is_some_and(|value| {
        !value.is_finite()
            || !(-10.0..=54.0).contains(&value)
            || (value * 10.0 - (value * 10.0).round()).abs() > 1e-8
    }) {
        return Err(ApiError::BadRequest(
            "handicap must be between -10 and 54 with at most one decimal".into(),
        ));
    }
    let result = profile::update_details(
        &state.pool,
        session.principal.session_id,
        profile::DetailsChange {
            version: input.version,
            player_updated_at: input.player_updated_at,
            display_name,
            handicap: input.handicap,
            reason,
        },
    )
    .await
    .map_err(map_error)?;
    for tournament in result.tournaments {
        state.notify("tournament", tournament, tournament);
    }
    Ok(Json(result.profile))
}

async fn verify_current(
    state: &AppState,
    session: &AuthenticatedSession,
    headers: &HeaderMap,
    password: String,
) -> ApiResult<(String, i64)> {
    state.rate_limiter.check(
        RateLimitRoute::ProfileCredentials,
        state.proxy_trust.client_identity(headers),
        session.principal.user_id.as_bytes(),
    )?;
    validate_password_length(&password).map_err(|_| map_error(ProfileError::IncorrectPassword))?;
    let credential = profile::credential(&state.pool, session.principal.user_id)
        .await?
        .ok_or_else(|| map_error(ProfileError::IncorrectPassword))?;
    if !verify_password_bounded(password, credential.0.clone())
        .await
        .map_err(|_| ApiError::Internal)?
    {
        return Err(map_error(ProfileError::IncorrectPassword));
    }
    Ok(credential)
}
async fn update_username(
    State(state): State<Arc<AppState>>,
    MutationSession(session): MutationSession,
    headers: HeaderMap,
    input: Result<Json<UsernameRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let Json(input) = input.map_err(input_error)?;
    let username = normalize_and_validate_username(&input.username)
        .map_err(|e| ApiError::BadRequest(e.into()))?;
    let verified = verify_current(&state, &session, &headers, input.current_password).await?;
    profile::update_credential(
        &state.pool,
        session.principal.session_id,
        input.version,
        &verified,
        CredentialChange::Username(username),
    )
    .await
    .map_err(map_error)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn update_password(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    MutationSession(session): MutationSession,
    headers: HeaderMap,
    input: Result<Json<PasswordRequest>, JsonRejection>,
) -> ApiResult<(CookieJar, StatusCode)> {
    let Json(input) = input.map_err(input_error)?;
    validate_password_length(&input.new_password).map_err(|e| ApiError::BadRequest(e.into()))?;
    let verified = verify_current(&state, &session, &headers, input.current_password).await?;
    let hash = hash_password_bounded(input.new_password)
        .await
        .map_err(|_| ApiError::Internal)?;
    profile::update_credential(
        &state.pool,
        session.principal.session_id,
        input.version,
        &verified,
        CredentialChange::Password(hash),
    )
    .await
    .map_err(map_error)?;
    let mut cookie = session_cookie(&state, String::new(), Utc::now());
    cookie.make_removal();
    Ok((jar.remove(cookie), StatusCode::NO_CONTENT))
}
