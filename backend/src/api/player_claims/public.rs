use super::Error;
use crate::{
    AppState,
    api::auth::session_cookie,
    auth::{
        SESSION_COOKIE_NAME, derive_csrf_token, generate_session_token, hash_password_bounded,
        hash_session_token,
    },
    domain::{
        accounts::{normalize_and_validate_username, validate_password_length},
        invitations::valid_token_shape,
    },
    error::ApiError,
    rate_limit::RateLimitRoute,
    repositories::{
        auth,
        player_claims::{self, ClaimError, ClaimPreview, RegisterParams},
    },
};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Token {
    token: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    token: String,
    account: Account,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Account {
    username: String,
    password: String,
}
async fn validate(
    s: &AppState,
    id: &str,
    headers: &HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
    route: RateLimitRoute,
) -> Result<(Uuid, Value, ClaimPreview), Error> {
    // Malformed identifiers share a bounded client quota too.
    let parsed_id = Uuid::parse_str(id);
    s.rate_limiter
        .check(
            route,
            s.proxy_trust.client_identity(headers),
            parsed_id
                .as_ref()
                .map(Uuid::as_bytes)
                .map(|b| b.as_slice())
                .unwrap_or(b"invalid-claim-id"),
        )
        .map_err(ApiError::from)?;
    let id = parsed_id.map_err(|_| ClaimError::Invalid)?;
    let Json(value) =
        input.map_err(|_| ApiError::BadRequest("request must contain a token".into()))?;
    let token = value
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::BadRequest("request must contain a string token".into()))?;
    if !valid_token_shape(token) {
        return Err(ClaimError::Invalid.into());
    }
    let preview = player_claims::preview(&s.pool, id, token).await?;
    Ok((id, value, preview))
}
pub(super) async fn preview(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
) -> Result<Json<ClaimPreview>, Error> {
    let (_, value, preview) =
        validate(&s, &id, &headers, input, RateLimitRoute::PlayerClaimPreview).await?;
    let input: Token = serde_json::from_value(value)
        .map_err(|_| ApiError::BadRequest("request must contain only token".into()))?;
    let _ = input.token;
    Ok(Json(preview))
}
pub(super) async fn register(
    State(s): State<Arc<AppState>>,
    jar: CookieJar,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
) -> Result<impl IntoResponse, Error> {
    let (id, value, _) = validate(
        &s,
        &id,
        &headers,
        input,
        RateLimitRoute::PlayerClaimRegister,
    )
    .await?;
    let input: Registration = serde_json::from_value(value)
        .map_err(|_| ApiError::BadRequest("request must contain token and account".into()))?;
    if let Some(cookie) = jar.get(SESSION_COOKIE_NAME)
        && auth::find_active_session(&s.pool, &hash_session_token(cookie.value()))
            .await
            .map_err(ApiError::Database)?
            .is_some()
    {
        return Err(ApiError::DomainConflict {
            code: "already_authenticated",
            message: "sign out before claiming an account",
        }
        .into());
    }
    let username = normalize_and_validate_username(&input.account.username)
        .map_err(|_| ApiError::BadRequest("account.username is invalid".into()))?;
    validate_password_length(&input.account.password).map_err(|_| {
        ApiError::BadRequest("account.password must be between 12 and 128 bytes".into())
    })?;
    let hash = hash_password_bounded(input.account.password)
        .await
        .map_err(|_| ApiError::Internal)?;
    let session_token = generate_session_token().map_err(|_| ApiError::Internal)?;
    let expires = Utc::now() + Duration::hours(s.auth.session_ttl_hours);
    let created = player_claims::register(
        &s.pool,
        RegisterParams {
            id,
            token: &input.token,
            username: &username,
            password_hash: &hash,
            session_token_hash: &hash_session_token(&session_token),
            session_expires_at: expires,
        },
    )
    .await?;
    let session = created.session.response(derive_csrf_token(&session_token));
    s.notify("tournament", created.tournament_id, created.tournament_id);
    Ok((
        StatusCode::CREATED,
        jar.add(session_cookie(&s, session_token, expires)),
        Json(
            json!({"tournament_id":created.tournament_id,"player_id":created.player_id,"session":session}),
        ),
    ))
}
