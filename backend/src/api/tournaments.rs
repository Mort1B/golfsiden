use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{StatusCode, header::CACHE_CONTROL},
    response::IntoResponse,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    api::{
        auth::{AuthenticatedSession, MutationSession},
        authorization::map_authorization_error,
    },
    domain::models::TournamentHandicapCorrection,
    error::{ApiError, ApiResult},
    repositories::tournaments::{self, TournamentMutationError},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tournaments", get(list))
        .route("/api/me/tournaments", get(list_mine))
        .route("/api/tournaments/{tournament_id}", get(get_one))
        .route(
            "/api/tournaments/{tournament_id}/start",
            axum::routing::post(start),
        )
        .route(
            "/api/tournaments/{tournament_id}/counted-rounds",
            axum::routing::patch(update_counted_rounds),
        )
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateCountedRounds {
    counted_rounds: i16,
    expected_tournament_updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartTournament {
    expected_tournament_updated_at: DateTime<Utc>,
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

async fn update_counted_rounds(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
    input: Result<Json<UpdateCountedRounds>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest(
            "request must contain only counted_rounds and expected_tournament_updated_at"
                .to_owned(),
        )
    })?;
    if !(1..=30).contains(&input.counted_rounds) {
        return Err(ApiError::BadRequest(
            "counted_rounds must be between 1 and the tournament round count".to_owned(),
        ));
    }
    let result = tournaments::update_counted_rounds_authorized(
        &state.pool,
        authenticated.principal.session_id,
        tournament_id,
        input.counted_rounds,
        input.expected_tournament_updated_at,
    )
    .await
    .map_err(map_mutation_error)?;
    if result.changed {
        state.notify("tournament", tournament_id);
    }
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(result.tournament),
    ))
}

async fn start(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
    input: Result<Json<StartTournament>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest("request must contain only expected_tournament_updated_at".to_owned())
    })?;
    let result = tournaments::start_authorized(
        &state.pool,
        authenticated.principal.session_id,
        tournament_id,
        input.expected_tournament_updated_at,
    )
    .await
    .map_err(map_mutation_error)?;
    if result.changed {
        state.notify("tournament", tournament_id);
    }
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(result.tournament),
    ))
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
        TournamentMutationError::CountedRoundsInvalid => ApiError::BadRequest(
            "counted_rounds must be between 1 and the tournament round count".to_owned(),
        ),
        TournamentMutationError::ConfigurationLocked => ApiError::DomainConflict {
            code: "tournament_configuration_locked",
            message: "tournament configuration is locked after round opening",
        },
        TournamentMutationError::ConfigurationStale => ApiError::DomainConflict {
            code: "tournament_configuration_stale",
            message: "tournament configuration changed; refresh and try again",
        },
        TournamentMutationError::StartNotReady => ApiError::DomainConflict {
            code: "tournament_start_not_ready",
            message: "tournament requires a complete draft round plan and an active entrant",
        },
        TournamentMutationError::StartInvalidState => ApiError::DomainConflict {
            code: "tournament_start_invalid_state",
            message: "tournament cannot be started from its current state",
        },
        TournamentMutationError::StartStale => ApiError::DomainConflict {
            code: "tournament_start_stale",
            message: "tournament changed; refresh and try again",
        },
        TournamentMutationError::Authorization(error) => map_authorization_error(error),
        TournamentMutationError::Database(error) => ApiError::Database(error),
    }
}
