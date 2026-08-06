use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use chrono::NaiveTime;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    domain::models::{TeamMember, TeamWithMembers},
    error::{ApiError, ApiResult, require_non_empty},
    repositories::{rounds, teams},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/rounds/{round_id}/teams", get(list).post(create))
        .route("/api/teams/{team_id}/members", post(assign_player))
        .route(
            "/api/teams/{team_id}/members/{player_id}",
            delete(remove_player),
        )
}

#[derive(Deserialize)]
struct CreateTeam {
    name: String,
    starting_hole: Option<i16>,
    tee_time: Option<NaiveTime>,
}

#[derive(Deserialize)]
struct AssignPlayer {
    player_id: Uuid,
    display_order: Option<i16>,
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<Vec<TeamWithMembers>>> {
    if rounds::get(&state.pool, round_id).await?.is_none() {
        return Err(ApiError::NotFound);
    }
    Ok(Json(teams::list(&state.pool, round_id).await?))
}

async fn create(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    Json(input): Json<CreateTeam>,
) -> ApiResult<impl IntoResponse> {
    require_non_empty(&input.name, "name")?;
    if let Some(hole) = input.starting_hole
        && !(1..=36).contains(&hole)
    {
        return Err(ApiError::BadRequest(
            "starting_hole must be between 1 and 36".to_owned(),
        ));
    }
    let team = teams::create(
        &state.pool,
        round_id,
        &input.name,
        input.starting_hole,
        input.tee_time,
    )
    .await?;
    state.notify("round", round_id);
    Ok((StatusCode::CREATED, Json(team)))
}

async fn assign_player(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    Json(input): Json<AssignPlayer>,
) -> ApiResult<impl IntoResponse> {
    let member: TeamMember =
        teams::assign_player(&state.pool, team_id, input.player_id, input.display_order).await?;
    state.notify("team", team_id);
    Ok((StatusCode::CREATED, Json(member)))
}

async fn remove_player(
    State(state): State<Arc<AppState>>,
    Path((team_id, player_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    if !teams::remove_player(&state.pool, team_id, player_id).await? {
        return Err(ApiError::NotFound);
    }
    state.notify("team", team_id);
    Ok(StatusCode::NO_CONTENT)
}
