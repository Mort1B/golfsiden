use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    api::auth::{AuthenticatedSession, MutationSession},
    auth::{generate_invitation_token, hash_invitation_token},
    domain::invitations::validate_issue_policy,
    error::ApiError,
    repositories::invitations::{self, InvitationMetadata},
};

use super::{InvitationApiError, parse_management_id};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IssueRequest {
    expires_at: DateTime<Utc>,
    max_uses: NullableMaximum,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct NullableMaximum(Option<i32>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RotateRequest {}

#[derive(Serialize)]
pub(super) struct InvitationWithToken {
    #[serde(flatten)]
    invitation: InvitationMetadata,
    token: String,
}

pub(super) async fn list(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<String>,
    authenticated: AuthenticatedSession,
) -> Result<Json<Vec<InvitationMetadata>>, InvitationApiError> {
    let tournament_id = parse_management_id(&tournament_id)?;
    Ok(Json(
        invitations::list(
            &state.pool,
            authenticated.principal.session_id,
            tournament_id,
        )
        .await?,
    ))
}

pub(super) async fn issue(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<String>,
    MutationSession(authenticated): MutationSession,
    input: Result<Json<IssueRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<InvitationWithToken>), InvitationApiError> {
    let tournament_id = parse_management_id(&tournament_id)?;
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest("request must contain expires_at and nullable max_uses".to_owned())
    })?;
    validate_issue_policy(input.expires_at, input.max_uses.0, Utc::now())
        .map_err(|message| ApiError::BadRequest(message.to_owned()))?;
    let invitation_id = Uuid::new_v4();
    let token = generate_invitation_token().map_err(|_| ApiError::Internal)?;
    let token_hash = hash_invitation_token(&token);
    let invitation = invitations::issue(
        &state.pool,
        invitations::admin::IssueParams {
            session_id: authenticated.principal.session_id,
            tournament_id,
            invitation_id,
            token_hash: &token_hash,
            expires_at: input.expires_at,
            max_uses: input.max_uses.0,
        },
    )
    .await?;
    state.notify("invitation", tournament_id);
    Ok((
        StatusCode::CREATED,
        Json(InvitationWithToken { invitation, token }),
    ))
}

pub(super) async fn rotate(
    State(state): State<Arc<AppState>>,
    Path((tournament_id, invitation_id)): Path<(String, String)>,
    MutationSession(authenticated): MutationSession,
    input: Result<Json<RotateRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<InvitationWithToken>), InvitationApiError> {
    let tournament_id = parse_management_id(&tournament_id)?;
    let invitation_id = parse_management_id(&invitation_id)?;
    let Json(_input) =
        input.map_err(|_| ApiError::BadRequest("request must be an empty object".to_owned()))?;
    let token = generate_invitation_token().map_err(|_| ApiError::Internal)?;
    let token_hash = hash_invitation_token(&token);
    let invitation = invitations::rotate(
        &state.pool,
        invitations::admin::RotateParams {
            session_id: authenticated.principal.session_id,
            tournament_id,
            predecessor_id: invitation_id,
            successor_id: Uuid::new_v4(),
            token_hash: &token_hash,
        },
    )
    .await?;
    state.notify("invitation", tournament_id);
    Ok((
        StatusCode::CREATED,
        Json(InvitationWithToken { invitation, token }),
    ))
}

pub(super) async fn revoke(
    State(state): State<Arc<AppState>>,
    Path((tournament_id, invitation_id)): Path<(String, String)>,
    MutationSession(authenticated): MutationSession,
) -> Result<StatusCode, InvitationApiError> {
    let tournament_id = parse_management_id(&tournament_id)?;
    let invitation_id = parse_management_id(&invitation_id)?;
    invitations::revoke(
        &state.pool,
        authenticated.principal.session_id,
        tournament_id,
        invitation_id,
    )
    .await?;
    state.notify("invitation", tournament_id);
    Ok(StatusCode::NO_CONTENT)
}
