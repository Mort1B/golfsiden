use super::*;
use crate::{
    auth::hash_password_bounded,
    domain::{accounts::validate_password_length, password_recovery::RecoveryToken},
    error::ApiResult,
    rate_limit::RateLimitRoute,
    repositories::password_recovery,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreviewRequest {
    token: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RedeemRequest {
    token: String,
    new_password: String,
    confirm_password: String,
}
#[derive(Serialize)]
pub(super) struct Preview {
    id: Uuid,
    expires_at: DateTime<Utc>,
}
pub(super) async fn preview(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<PreviewRequest>, JsonRejection>,
) -> ApiResult<Json<Preview>> {
    let id: Uuid = id.parse().map_err(|_| map_error(RecoveryError::Invalid))?;
    state.rate_limiter.check(
        RateLimitRoute::RecoveryPreview,
        state.proxy_trust.client_identity(&headers),
        id.as_bytes(),
    )?;
    let Json(input) = input.map_err(input_error)?;
    let token =
        RecoveryToken::parse(input.token).ok_or_else(|| map_error(RecoveryError::Invalid))?;
    let grant = password_recovery::preview(&state.pool, id, &token)
        .await
        .map_err(map_error)?;
    Ok(Json(Preview {
        id: grant.id,
        expires_at: grant.expires_at,
    }))
}
pub(super) async fn redeem(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<RedeemRequest>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let id: Uuid = id.parse().map_err(|_| map_error(RecoveryError::Invalid))?;
    state.rate_limiter.check(
        RateLimitRoute::RecoveryRedeem,
        state.proxy_trust.client_identity(&headers),
        id.as_bytes(),
    )?;
    let Json(input) = input.map_err(input_error)?;
    let token =
        RecoveryToken::parse(input.token).ok_or_else(|| map_error(RecoveryError::Invalid))?;
    validate_password_length(&input.new_password).map_err(|e| ApiError::BadRequest(e.into()))?;
    if input.new_password != input.confirm_password {
        return Err(ApiError::BadRequest("passwords must match".into()));
    }
    // A cheap valid-link check precedes expensive work; final redemption repeats
    // every check under locks after bounded hashing, with no transaction held.
    password_recovery::preview(&state.pool, id, &token)
        .await
        .map_err(map_error)?;
    let hash = hash_password_bounded(input.new_password)
        .await
        .map_err(|_| ApiError::Internal)?;
    password_recovery::redeem(&state.pool, id, &token, &hash)
        .await
        .map_err(map_error)?;
    Ok(StatusCode::NO_CONTENT)
}
