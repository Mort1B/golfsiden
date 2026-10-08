use super::Error;
use crate::{
    AppState,
    api::auth::{AuthenticatedSession, MutationSession},
    auth::{generate_invitation_token, hash_invitation_token},
    error::ApiError,
    repositories::player_claims::{self, AccountState, Grant},
};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Create {
    display_name: String,
    handicap_index: f64,
}
#[derive(Serialize)]
pub(super) struct WithToken {
    #[serde(flatten)]
    grant: Grant,
    token: String,
}
pub(super) async fn list(
    State(s): State<Arc<AppState>>,
    Path(t): Path<Uuid>,
    auth: AuthenticatedSession,
) -> Result<Json<Vec<AccountState>>, Error> {
    Ok(Json(
        player_claims::list(&s.pool, auth.principal.session_id, t).await?,
    ))
}
pub(super) async fn create(
    State(s): State<Arc<AppState>>,
    Path(t): Path<Uuid>,
    MutationSession(auth): MutationSession,
    input: Result<Json<Create>, JsonRejection>,
) -> Result<(StatusCode, Json<WithToken>), Error> {
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest("request must contain display_name and handicap_index".into())
    })?;
    if input.display_name.trim().is_empty()
        || input.display_name.len() > 120
        || input.display_name.contains('\0')
        || !input.handicap_index.is_finite()
        || !(-10.0..=54.0).contains(&input.handicap_index)
    {
        return Err(
            ApiError::BadRequest("invalid player name or handicap (-10 to 54)".into()).into(),
        );
    }
    let token = generate_invitation_token().map_err(|_| ApiError::Internal)?;
    let grant = player_claims::create(
        &s.pool,
        auth.principal.session_id,
        t,
        input.display_name.trim(),
        input.handicap_index,
        &hash_invitation_token(&token),
    )
    .await?;
    s.notify("tournament", t, t);
    Ok((StatusCode::CREATED, Json(WithToken { grant, token })))
}
pub(super) async fn reissue(
    State(s): State<Arc<AppState>>,
    Path((t, p)): Path<(Uuid, Uuid)>,
    MutationSession(auth): MutationSession,
) -> Result<(StatusCode, Json<WithToken>), Error> {
    let token = generate_invitation_token().map_err(|_| ApiError::Internal)?;
    let grant = player_claims::reissue(
        &s.pool,
        auth.principal.session_id,
        t,
        p,
        &hash_invitation_token(&token),
    )
    .await?;
    s.notify("tournament", t, t);
    Ok((StatusCode::CREATED, Json(WithToken { grant, token })))
}
pub(super) async fn revoke(
    State(s): State<Arc<AppState>>,
    Path((t, p)): Path<(Uuid, Uuid)>,
    MutationSession(auth): MutationSession,
) -> Result<StatusCode, Error> {
    player_claims::revoke(&s.pool, auth.principal.session_id, t, p).await?;
    s.notify("tournament", t, t);
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn withdraw(
    State(s): State<Arc<AppState>>,
    Path((t, p)): Path<(Uuid, Uuid)>,
    MutationSession(auth): MutationSession,
) -> Result<StatusCode, Error> {
    player_claims::withdraw(&s.pool, auth.principal.session_id, t, p).await?;
    s.notify("tournament", t, t);
    Ok(StatusCode::NO_CONTENT)
}
