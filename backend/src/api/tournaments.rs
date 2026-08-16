use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{StatusCode, header::CACHE_CONTROL},
    response::IntoResponse,
    routing::get,
};
use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    api::{
        auth::{AuthenticatedSession, MutationSession, PlatformAdminSession},
        authorization::map_authorization_error,
    },
    domain::models::{ScoringMode, TournamentHandicapCorrection, TournamentStatus},
    error::{ApiError, ApiResult, require_non_empty},
    repositories::tournaments::{self, TournamentMutationError},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tournaments", get(list).post(create))
        .route("/api/me/tournaments", get(list_mine))
        .route("/api/tournaments/{tournament_id}", get(get_one))
        .route(
            "/api/tournaments/{tournament_id}/players",
            get(list_players).post(add_player),
        )
        .route(
            "/api/tournaments/{tournament_id}/players/{player_id}/handicap-corrections",
            axum::routing::post(change_player_handicap),
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateTournament {
    name: String,
    #[serde(default)]
    description: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
    number_of_rounds: i16,
    #[serde(default = "default_status")]
    status: TournamentStatus,
    #[serde(default = "default_scoring_mode")]
    scoring_mode: ScoringMode,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddTournamentPlayer {
    player_id: Uuid,
    tournament_handicap: Option<f64>,
    seed: Option<i16>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeTournamentHandicap {
    handicap_index: f64,
    reason: String,
}

fn default_status() -> TournamentStatus {
    TournamentStatus::Draft
}
fn default_scoring_mode() -> ScoringMode {
    ScoringMode::Combined
}

async fn list(
    State(state): State<Arc<AppState>>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    let tournaments =
        tournaments::list_for_member(&state.pool, authenticated.principal.user_id).await?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(tournaments)))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    let tournament = tournaments::get_for_member(&state.pool, authenticated.principal.user_id, id)
        .await
        .map_err(map_authorization_error)?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(tournament)))
}

async fn create(
    State(state): State<Arc<AppState>>,
    PlatformAdminSession(authenticated): PlatformAdminSession,
    Json(input): Json<CreateTournament>,
) -> ApiResult<impl IntoResponse> {
    require_non_empty(&input.name, "name")?;
    if input.end_date < input.start_date {
        return Err(ApiError::BadRequest(
            "end_date must not be before start_date".to_owned(),
        ));
    }
    if !(1..=30).contains(&input.number_of_rounds) {
        return Err(ApiError::BadRequest(
            "number_of_rounds must be between 1 and 30".to_owned(),
        ));
    }
    let tournament = tournaments::create_platform_authorized(
        &state.pool,
        authenticated.principal.session_id,
        &input.name,
        &input.description,
        input.start_date,
        input.end_date,
        input.number_of_rounds,
        input.status,
        input.scoring_mode,
    )
    .await
    .map_err(map_mutation_error)?;
    state.notify("tournament", tournament.id);
    Ok((StatusCode::CREATED, Json(tournament)))
}

async fn list_mine(
    State(state): State<Arc<AppState>>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(tournaments::list_for_user(&state.pool, authenticated.principal.user_id).await?),
    ))
}

async fn list_players(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<impl IntoResponse> {
    let roster =
        tournaments::list_players_for_member(&state.pool, authenticated.principal.user_id, id)
            .await
            .map_err(map_authorization_error)?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(roster)))
}

async fn add_player(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
    Json(input): Json<AddTournamentPlayer>,
) -> ApiResult<impl IntoResponse> {
    if let Some(handicap) = input.tournament_handicap
        && (!handicap.is_finite() || !(-10.0..=54.0).contains(&handicap))
    {
        return Err(ApiError::BadRequest(
            "tournament_handicap must be between -10.0 and 54.0".to_owned(),
        ));
    }
    let player = tournaments::add_player_authorized(
        &state.pool,
        authenticated.principal.session_id,
        id,
        input.player_id,
        input.tournament_handicap,
        input.seed,
    )
    .await
    .map_err(map_mutation_error)?;
    state.notify("tournament", id);
    Ok((StatusCode::CREATED, Json(player)))
}

async fn change_player_handicap(
    State(state): State<Arc<AppState>>,
    Path((tournament_id, player_id)): Path<(Uuid, Uuid)>,
    MutationSession(authenticated): MutationSession,
    input: Result<Json<ChangeTournamentHandicap>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest("request must contain only handicap_index and reason".to_owned())
    })?;
    if !input.handicap_index.is_finite() || !(-10.0..=54.0).contains(&input.handicap_index) {
        return Err(ApiError::BadRequest(
            "handicap must be between -10.0 and 54.0".to_owned(),
        ));
    }
    let reason = input.reason.trim();
    if reason.is_empty() || reason.len() > 500 {
        return Err(ApiError::BadRequest(
            "reason must contain between 1 and 500 bytes".to_owned(),
        ));
    }
    let correction: TournamentHandicapCorrection = tournaments::change_player_handicap_authorized(
        &state.pool,
        authenticated.principal.session_id,
        tournament_id,
        player_id,
        input.handicap_index,
        reason,
    )
    .await
    .map_err(map_mutation_error)?;
    state.notify("tournament", tournament_id);
    Ok((StatusCode::CREATED, Json(correction)))
}

fn map_mutation_error(error: TournamentMutationError) -> ApiError {
    match error {
        TournamentMutationError::NotFound => ApiError::NotFound,
        TournamentMutationError::HandicapLocked => ApiError::DomainConflict {
            code: "tournament_handicap_locked",
            message: "tournament handicap is locked after round opening",
        },
        TournamentMutationError::HandicapUnchanged => ApiError::DomainConflict {
            code: "tournament_handicap_unchanged",
            message: "tournament handicap is unchanged",
        },
        TournamentMutationError::Authorization(error) => map_authorization_error(error),
        TournamentMutationError::Database(error) => ApiError::Database(error),
    }
}
