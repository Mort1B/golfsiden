use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    domain::models::{HandicapHistoryEntry, Player},
    error::{ApiError, ApiResult, require_non_empty},
    repositories::players,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/players", get(list).post(create))
        .route(
            "/api/players/{player_id}",
            get(get_one).patch(update).delete(deactivate),
        )
        .route(
            "/api/players/{player_id}/handicaps",
            get(handicap_history).post(change_handicap),
        )
}

#[derive(Deserialize)]
struct CreatePlayer {
    display_name: String,
    current_handicap_index: f64,
    email: Option<String>,
    profile_image_ref: Option<String>,
}

#[derive(Deserialize)]
struct UpdatePlayer {
    display_name: Option<String>,
    email: Option<String>,
    profile_image_ref: Option<String>,
    active: Option<bool>,
}

#[derive(Deserialize)]
struct ChangeHandicap {
    handicap_index: f64,
    changed_by: Option<Uuid>,
    reason: Option<String>,
}

async fn list(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Player>>> {
    Ok(Json(players::list(&state.pool).await?))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Player>> {
    Ok(Json(
        players::get(&state.pool, id)
            .await?
            .ok_or(ApiError::NotFound)?,
    ))
}

async fn create(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreatePlayer>,
) -> ApiResult<impl IntoResponse> {
    require_non_empty(&input.display_name, "display_name")?;
    validate_handicap(input.current_handicap_index)?;
    let player = players::create(
        &state.pool,
        &input.display_name,
        input.current_handicap_index,
        input.email.as_deref(),
        input.profile_image_ref.as_deref(),
    )
    .await?;
    state.notify("player", player.id);
    Ok((StatusCode::CREATED, Json(player)))
}

async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdatePlayer>,
) -> ApiResult<Json<Player>> {
    if let Some(name) = &input.display_name {
        require_non_empty(name, "display_name")?;
    }
    let player = players::update(
        &state.pool,
        id,
        input.display_name.as_deref(),
        input.email.as_deref(),
        input.profile_image_ref.as_deref(),
        input.active,
    )
    .await?
    .ok_or(ApiError::NotFound)?;
    state.notify("player", id);
    Ok(Json(player))
}

async fn deactivate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    if !players::deactivate(&state.pool, id).await? {
        return Err(ApiError::NotFound);
    }
    state.notify("player", id);
    Ok(StatusCode::NO_CONTENT)
}

async fn change_handicap(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(input): Json<ChangeHandicap>,
) -> ApiResult<impl IntoResponse> {
    validate_handicap(input.handicap_index)?;
    let entry = players::change_handicap(
        &state.pool,
        id,
        input.handicap_index,
        input.changed_by,
        input.reason.as_deref(),
    )
    .await?
    .ok_or(ApiError::NotFound)?;
    state.notify("player", id);
    Ok((StatusCode::CREATED, Json(entry)))
}

async fn handicap_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<HandicapHistoryEntry>>> {
    if players::get(&state.pool, id).await?.is_none() {
        return Err(ApiError::NotFound);
    }
    Ok(Json(players::handicap_history(&state.pool, id).await?))
}

fn validate_handicap(value: f64) -> ApiResult<()> {
    if !value.is_finite() || !(-10.0..=54.0).contains(&value) {
        return Err(ApiError::BadRequest(
            "handicap must be between -10.0 and 54.0".to_owned(),
        ));
    }
    Ok(())
}
