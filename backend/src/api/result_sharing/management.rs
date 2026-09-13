use super::*;
use crate::{
    api::auth::{AuthenticatedSession, MutationSession},
    domain::result_sharing::ResultShareToken,
    error::ApiResult,
    rate_limit::RateLimitRoute,
    repositories::result_sharing::{self, GrantMetadata},
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IssueRequest {
    #[serde(deserialize_with = "required_nullable")]
    expected_grant_id: Option<Uuid>,
}
fn required_nullable<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Uuid>, D::Error> {
    Option::<Uuid>::deserialize(deserializer)
}
#[derive(Serialize)]
pub(super) struct Status {
    tournament_id: Uuid,
    grant: Option<GrantMetadata>,
}
#[derive(Serialize)]
pub(super) struct Issued {
    tournament_id: Uuid,
    grant: GrantMetadata,
    token: String,
}
fn tournament_id(value: String) -> ApiResult<Uuid> {
    value
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid tournament identifier".into()))
}
pub(super) async fn status(
    State(state): State<Arc<AppState>>,
    AuthenticatedSession { principal, .. }: AuthenticatedSession,
    Path(tournament): Path<String>,
) -> ApiResult<Json<Status>> {
    let tournament = tournament_id(tournament)?;
    let grant = result_sharing::status(&state.pool, principal.session_id, tournament)
        .await
        .map_err(map_error)?;
    Ok(Json(Status {
        tournament_id: tournament,
        grant,
    }))
}
pub(super) async fn issue(
    State(state): State<Arc<AppState>>,
    MutationSession(session): MutationSession,
    Path(tournament): Path<String>,
    headers: HeaderMap,
    input: Result<Json<IssueRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<Issued>)> {
    let tournament = tournament_id(tournament)?;
    state.rate_limiter.check(
        RateLimitRoute::ResultShareAdmin,
        state.proxy_trust.client_identity(&headers),
        session.principal.user_id.as_bytes(),
    )?;
    let Json(input) = input.map_err(input_error)?;
    let token = ResultShareToken::generate().map_err(|_| ApiError::Internal)?;
    let grant = result_sharing::issue(
        &state.pool,
        session.principal.session_id,
        tournament,
        input.expected_grant_id,
        &token.hash(),
    )
    .await
    .map_err(map_error)?;
    state.notify("tournament", tournament, tournament);
    Ok((
        StatusCode::CREATED,
        Json(Issued {
            tournament_id: tournament,
            grant,
            token: token.expose().to_owned(),
        }),
    ))
}
pub(super) async fn revoke(
    State(state): State<Arc<AppState>>,
    MutationSession(session): MutationSession,
    Path((tournament, grant)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let tournament = tournament_id(tournament)?;
    let grant = grant
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid result link identifier".into()))?;
    state.rate_limiter.check(
        RateLimitRoute::ResultShareAdmin,
        state.proxy_trust.client_identity(&headers),
        session.principal.user_id.as_bytes(),
    )?;
    if result_sharing::revoke(&state.pool, session.principal.session_id, tournament, grant)
        .await
        .map_err(map_error)?
    {
        state.notify("tournament", tournament, tournament);
    }
    Ok(StatusCode::NO_CONTENT)
}
