use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware,
    response::{IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    api::auth::session_cookie,
    auth::{
        SESSION_COOKIE_NAME, SessionResponse, derive_csrf_token, generate_invitation_token,
        generate_session_token, hash_invitation_token, hash_password_bounded, hash_session_token,
    },
    domain::{
        models::{Round, ScoringFormat, Tournament, TournamentRole},
        onboarding::{self, OnboardingInput, RoundInput},
    },
    error::{ApiError, ApiResult},
    repositories::{
        auth,
        onboarding::{self as onboarding_repository, OnboardingRepositoryError},
    },
};

const MAX_ONBOARDING_BODY_BYTES: usize = 64 * 1024;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/onboarding/tournaments", post(create))
        .layer(DefaultBodyLimit::max(MAX_ONBOARDING_BODY_BYTES))
        .layer(middleware::map_response(no_store))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateOnboardingRequest {
    creator: CreatorRequest,
    tournament: TournamentRequest,
    rounds: Vec<RoundRequest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatorRequest {
    account: AccountRequest,
    player: PlayerRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerRequest {
    display_name: String,
    handicap_index: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TournamentRequest {
    name: String,
    description: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoundRequest {
    round_number: i16,
    name: String,
    round_date: NaiveDate,
    scoring_format: ScoringFormat,
}

#[derive(Serialize)]
struct OnboardingResponse {
    tournament: Tournament,
    rounds: Vec<Round>,
    session: SessionResponse,
    creator: CreatorResponse,
    invitation: InvitationResponse,
}

#[derive(Serialize)]
struct CreatorResponse {
    user_id: Uuid,
    player_id: Uuid,
    tournament_role: TournamentRole,
}

#[derive(Serialize)]
struct InvitationResponse {
    id: Uuid,
    token: String,
    expires_at: chrono::DateTime<Utc>,
    max_uses: Option<u32>,
}

async fn create(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    input: Result<Json<CreateOnboardingRequest>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest(
            "request must match the creator, tournament, and rounds contract".to_owned(),
        )
    })?;
    let mut input = onboarding::validate(input.into_domain(), Utc::now().date_naive())
        .map_err(|message| ApiError::BadRequest(message.to_owned()))?;

    if active_session_exists(&state, &jar).await? {
        return Err(ApiError::DomainConflict {
            code: "already_authenticated",
            message: "sign out before creating a first-time account",
        });
    }

    let password = std::mem::take(&mut input.password);
    let password_hash = hash_password_bounded(password)
        .await
        .map_err(|_| ApiError::Internal)?;
    let session_token = generate_session_token().map_err(|_| ApiError::Internal)?;
    let invitation_token = generate_invitation_token().map_err(|_| ApiError::Internal)?;
    let session_token_hash = hash_session_token(&session_token);
    let invitation_token_hash = hash_invitation_token(&invitation_token);
    let session_expires_at = Utc::now() + Duration::hours(state.auth.session_ttl_hours);

    let created = onboarding_repository::create(
        &state.pool,
        onboarding_repository::CreateOnboardingParams {
            input: &input,
            password_hash: &password_hash,
            session_token_hash: &session_token_hash,
            session_expires_at,
            invitation_token_hash: &invitation_token_hash,
        },
    )
    .await
    .map_err(map_repository_error)?;

    let csrf_token = derive_csrf_token(&session_token);
    let session_response = created.session.response(csrf_token);
    let user_id = created.session.user_id;
    let response = OnboardingResponse {
        tournament: created.tournament,
        rounds: created.rounds,
        session: session_response,
        creator: CreatorResponse {
            user_id,
            player_id: created.creator_player_id,
            tournament_role: created.creator_role,
        },
        invitation: InvitationResponse {
            id: created.invitation_id,
            token: invitation_token,
            expires_at: input.invitation_expires_at,
            max_uses: None,
        },
    };
    let tournament_id = response.tournament.id;
    let cookie = session_cookie(&state, session_token, session_expires_at);
    state.notify("tournament", tournament_id);
    Ok((StatusCode::CREATED, jar.add(cookie), Json(response)))
}

async fn active_session_exists(state: &AppState, jar: &CookieJar) -> ApiResult<bool> {
    let Some(token) = jar.get(SESSION_COOKIE_NAME).map(Cookie::value) else {
        return Ok(false);
    };
    Ok(
        auth::find_active_session(&state.pool, &hash_session_token(token))
            .await?
            .is_some(),
    )
}

fn map_repository_error(error: OnboardingRepositoryError) -> ApiError {
    match error {
        OnboardingRepositoryError::DuplicateUsername => ApiError::DomainConflict {
            code: "username_already_registered",
            message: "an account with this username already exists",
        },
        OnboardingRepositoryError::Database(error) => ApiError::Database(error),
    }
}

async fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

impl CreateOnboardingRequest {
    fn into_domain(self) -> OnboardingInput {
        OnboardingInput {
            username: self.creator.account.username,
            password: self.creator.account.password,
            display_name: self.creator.player.display_name,
            handicap_index: self.creator.player.handicap_index,
            tournament_name: self.tournament.name,
            description: self.tournament.description,
            start_date: self.tournament.start_date,
            end_date: self.tournament.end_date,
            rounds: self
                .rounds
                .into_iter()
                .map(|round| RoundInput {
                    round_number: round.round_number,
                    name: round.name,
                    round_date: round.round_date,
                    scoring_format: round.scoring_format,
                })
                .collect(),
        }
    }
}
