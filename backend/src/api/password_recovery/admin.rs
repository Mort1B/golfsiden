use super::*;
use crate::{
    api::auth::{AuthenticatedSession, MutationSession},
    auth::verify_password_bounded,
    domain::{accounts::validate_password_length, password_recovery::RecoveryToken},
    error::ApiResult,
    rate_limit::RateLimitRoute,
    repositories::{
        password_recovery::{self, AdminRequest},
        profile,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    current_password: String,
}
#[derive(Serialize)]
pub(super) struct Issued {
    id: Uuid,
    expires_at: DateTime<Utc>,
    reset_url: String,
}
async fn authorize(
    state: &AppState,
    session: AuthenticatedSession,
    headers: &HeaderMap,
    tournament: Uuid,
    player: Uuid,
    password: String,
) -> ApiResult<AdminRequest> {
    state.rate_limiter.check(
        RateLimitRoute::RecoveryAdmin,
        state.proxy_trust.client_identity(headers),
        session.principal.user_id.as_bytes(),
    )?;
    validate_password_length(&password).map_err(|_| map_error(RecoveryError::IncorrectPassword))?;
    let credential = profile::credential(&state.pool, session.principal.user_id)
        .await
        .map_err(|_| ApiError::Internal)?
        .ok_or_else(|| map_error(RecoveryError::IncorrectPassword))?;
    if !verify_password_bounded(password, credential.0.clone())
        .await
        .map_err(|_| ApiError::Internal)?
    {
        return Err(map_error(RecoveryError::IncorrectPassword));
    }
    Ok(AdminRequest {
        session_id: session.principal.session_id,
        user_id: session.principal.user_id,
        tournament_id: tournament,
        player_id: player,
        verified_password: credential,
    })
}
pub(super) async fn issue(
    State(state): State<Arc<AppState>>,
    MutationSession(session): MutationSession,
    Path((tournament, player)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Json<Request>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<Issued>)> {
    let tournament = tournament
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid tournament identifier".into()))?;
    let player = player
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid player identifier".into()))?;
    let Json(input) = input.map_err(input_error)?;
    let request = authorize(
        &state,
        session,
        &headers,
        tournament,
        player,
        input.current_password,
    )
    .await?;
    let origin = state
        .auth
        .recovery_origin
        .as_ref()
        .ok_or(ApiError::ServiceUnavailable)?;
    let token = RecoveryToken::generate().map_err(|_| ApiError::Internal)?;
    let grant = password_recovery::admin_issue(&state.pool, &request, &token.hash())
        .await
        .map_err(map_error)?;
    Ok((
        StatusCode::CREATED,
        Json(Issued {
            id: grant.id,
            expires_at: grant.expires_at,
            reset_url: origin.link(grant.id, &token),
        }),
    ))
}
pub(super) async fn revoke(
    State(state): State<Arc<AppState>>,
    MutationSession(session): MutationSession,
    Path((tournament, player)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Json<Request>, JsonRejection>,
) -> ApiResult<StatusCode> {
    let tournament = tournament
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid tournament identifier".into()))?;
    let player = player
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid player identifier".into()))?;
    let Json(input) = input.map_err(input_error)?;
    let request = authorize(
        &state,
        session,
        &headers,
        tournament,
        player,
        input.current_password,
    )
    .await?;
    password_recovery::admin_revoke(&state.pool, &request)
        .await
        .map_err(map_error)?;
    Ok(StatusCode::NO_CONTENT)
}
