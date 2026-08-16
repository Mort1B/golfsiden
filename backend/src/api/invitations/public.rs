use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    AppState,
    api::auth::{MutationSession, session_cookie},
    auth::{
        SESSION_COOKIE_NAME, SessionResponse, derive_csrf_token, generate_session_token,
        hash_password_bounded, hash_session_token,
    },
    domain::invitations::{RegistrationInput, valid_token_shape, validate_registration},
    error::ApiError,
    repositories::{
        auth,
        invitations::{self, InvitationError, InvitationPreview, JoinStatus},
    },
};

use super::{InvitationApiError, parse_public_id};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TokenRequest {
    token: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RegisterRequest {
    token: String,
    account: AccountRequest,
    player: PlayerRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerRequest {
    display_name: String,
    handicap_index: f64,
}

#[derive(Serialize)]
struct RegisterResponse {
    status: JoinStatus,
    tournament_id: Uuid,
    player_id: Uuid,
    session: SessionResponse,
}

pub(super) async fn preview(
    State(state): State<Arc<AppState>>,
    Path(invitation_id): Path<String>,
    input: Result<Json<Value>, JsonRejection>,
) -> Result<Json<InvitationPreview>, InvitationApiError> {
    let invitation_id = parse_public_id(&invitation_id)?;
    let Json(value) = input.map_err(invalid_token_body)?;
    authenticate_body_token(&state, invitation_id, &value).await?;
    let input: TokenRequest = strict_decode(value, "request must contain only token")?;
    Ok(Json(
        invitations::preview(&state.pool, invitation_id, &input.token, Utc::now()).await?,
    ))
}

pub(super) async fn register(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Path(invitation_id): Path<String>,
    input: Result<Json<Value>, JsonRejection>,
) -> Result<impl IntoResponse, InvitationApiError> {
    let invitation_id = parse_public_id(&invitation_id)?;
    let Json(value) = input.map_err(|_| {
        ApiError::BadRequest("request must contain token, account, and player objects".to_owned())
    })?;
    authenticate_body_token(&state, invitation_id, &value).await?;
    let input: RegisterRequest = strict_decode(
        value,
        "request must contain token, account, and player objects",
    )?;
    invitations::preview(&state.pool, invitation_id, &input.token, Utc::now()).await?;
    if active_session_exists(&state, &jar).await? {
        return Err(ApiError::DomainConflict {
            code: "already_authenticated",
            message: "sign out before registering a new account",
        }
        .into());
    }
    let mut registration = validate_registration(RegistrationInput {
        email: input.account.email,
        password: input.account.password,
        display_name: input.player.display_name,
        handicap_index: input.player.handicap_index,
    })
    .map_err(|message| ApiError::BadRequest(message.to_owned()))?;
    let password = std::mem::take(&mut registration.password);
    let password_hash = hash_password_bounded(password)
        .await
        .map_err(|_| ApiError::Internal)?;
    let session_token = generate_session_token().map_err(|_| ApiError::Internal)?;
    let session_token_hash = hash_session_token(&session_token);
    let session_expires_at = Utc::now() + Duration::hours(state.auth.session_ttl_hours);
    let created = invitations::register(
        &state.pool,
        invitations::RegisterParams {
            invitation_id,
            token: &input.token,
            registration: &registration,
            password_hash: &password_hash,
            session_token_hash: &session_token_hash,
            session_expires_at,
        },
    )
    .await?;
    let session = created.session.response(derive_csrf_token(&session_token));
    let cookie = session_cookie(&state, session_token, session_expires_at);
    state.notify("tournament", created.tournament_id);
    Ok((
        StatusCode::CREATED,
        jar.add(cookie),
        Json(RegisterResponse {
            status: JoinStatus::Joined,
            tournament_id: created.tournament_id,
            player_id: created.player_id,
            session,
        }),
    ))
}

pub(super) async fn accept(
    State(state): State<Arc<AppState>>,
    Path(invitation_id): Path<String>,
    MutationSession(authenticated): MutationSession,
    input: Result<Json<Value>, JsonRejection>,
) -> Result<impl IntoResponse, InvitationApiError> {
    let invitation_id = parse_public_id(&invitation_id)?;
    let Json(value) = input.map_err(invalid_token_body)?;
    authenticate_body_token(&state, invitation_id, &value).await?;
    let input: TokenRequest = strict_decode(value, "request must contain only token")?;
    let accepted = invitations::accept(
        &state.pool,
        authenticated.principal.session_id,
        invitation_id,
        &input.token,
    )
    .await?;
    if accepted.status == JoinStatus::Joined {
        state.notify("tournament", accepted.tournament_id);
    }
    let status = if accepted.status == JoinStatus::Joined {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(accepted)))
}

pub(super) async fn missing_id() -> InvitationApiError {
    InvitationError::Invalid.into()
}

fn require_token_shape(token: &str) -> Result<(), InvitationApiError> {
    if valid_token_shape(token) {
        Ok(())
    } else {
        Err(InvitationError::Invalid.into())
    }
}

async fn authenticate_body_token(
    state: &AppState,
    invitation_id: Uuid,
    value: &Value,
) -> Result<(), InvitationApiError> {
    let token = value
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::BadRequest("request must contain a string token".to_owned()))?;
    require_token_shape(token)?;
    invitations::authenticate_token(&state.pool, invitation_id, token).await?;
    Ok(())
}

fn strict_decode<T: DeserializeOwned>(
    value: Value,
    message: &'static str,
) -> Result<T, InvitationApiError> {
    serde_json::from_value(value).map_err(|_| ApiError::BadRequest(message.to_owned()).into())
}

fn invalid_token_body(_: JsonRejection) -> InvitationApiError {
    ApiError::BadRequest("request must contain only token".to_owned()).into()
}

async fn active_session_exists(state: &AppState, jar: &CookieJar) -> Result<bool, ApiError> {
    let Some(token) = jar.get(SESSION_COOKIE_NAME).map(Cookie::value) else {
        return Ok(false);
    };
    Ok(
        auth::find_active_session(&state.pool, &hash_session_token(token))
            .await?
            .is_some(),
    )
}
