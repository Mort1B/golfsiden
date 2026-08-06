mod completion;
mod lifecycle;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    domain::models::{Round, ScoringFormat},
    error::{ApiError, ApiResult, require_non_empty},
    repositories::{rounds, tournaments},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/tournaments/{tournament_id}/rounds",
            get(list).post(create),
        )
        .route("/api/rounds/{round_id}", get(get_one))
        .route(
            "/api/rounds/{round_id}/pairing-validation",
            get(lifecycle::pairing_validation),
        )
        .route(
            "/api/rounds/{round_id}/open",
            axum::routing::post(lifecycle::open),
        )
        .route(
            "/api/rounds/{round_id}/completion-validation",
            get(completion::validation),
        )
        .route(
            "/api/rounds/{round_id}/complete",
            axum::routing::post(completion::complete),
        )
        .route(
            "/api/rounds/{round_id}/lock",
            axum::routing::post(completion::lock),
        )
}

#[derive(Deserialize)]
struct CreateRound {
    round_number: i16,
    name: String,
    round_date: NaiveDate,
    course_id: Option<Uuid>,
    course_name: String,
    tee_id: Option<Uuid>,
    tee_name: String,
    #[serde(default = "default_holes")]
    number_of_holes: i16,
    #[serde(default = "default_true")]
    handicap_enabled: bool,
    #[serde(default = "default_allowance")]
    handicap_allowance_percent: i16,
    scoring_format: ScoringFormat,
}

fn default_holes() -> i16 {
    18
}

fn default_true() -> bool {
    true
}

fn default_allowance() -> i16 {
    100
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Round>>> {
    if tournaments::get(&state.pool, tournament_id)
        .await?
        .is_none()
    {
        return Err(ApiError::NotFound);
    }
    Ok(Json(rounds::list(&state.pool, tournament_id).await?))
}

async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
) -> ApiResult<Json<Round>> {
    Ok(Json(
        rounds::get(&state.pool, round_id)
            .await?
            .ok_or(ApiError::NotFound)?,
    ))
}

async fn create(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    Json(input): Json<CreateRound>,
) -> ApiResult<impl IntoResponse> {
    validate_create(&state, tournament_id, &input).await?;
    let round = rounds::create(
        &state.pool,
        rounds::CreateRoundParams {
            tournament_id,
            round_number: input.round_number,
            name: &input.name,
            round_date: input.round_date,
            course_id: input.course_id,
            course_name: &input.course_name,
            tee_id: input.tee_id,
            tee_name: &input.tee_name,
            number_of_holes: input.number_of_holes,
            handicap_enabled: input.handicap_enabled,
            handicap_allowance_percent: input.handicap_allowance_percent,
            scoring_format: input.scoring_format,
        },
    )
    .await?;
    state.notify("round", round.id);
    Ok((StatusCode::CREATED, Json(round)))
}

async fn validate_create(
    state: &AppState,
    tournament_id: Uuid,
    input: &CreateRound,
) -> ApiResult<()> {
    require_non_empty(&input.name, "name")?;
    require_non_empty(&input.course_name, "course_name")?;
    require_non_empty(&input.tee_name, "tee_name")?;
    let tournament = tournaments::get(&state.pool, tournament_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if input.round_number < 1 || input.round_number > tournament.number_of_rounds {
        return Err(ApiError::BadRequest(format!(
            "round_number must be between 1 and {}",
            tournament.number_of_rounds
        )));
    }
    if !(1..=36).contains(&input.number_of_holes) {
        return Err(ApiError::BadRequest(
            "number_of_holes must be between 1 and 36".to_owned(),
        ));
    }
    if !(0..=100).contains(&input.handicap_allowance_percent) {
        return Err(ApiError::BadRequest(
            "handicap_allowance_percent must be between 0 and 100".to_owned(),
        ));
    }
    match (input.course_id, input.tee_id) {
        (None, None) => {}
        (Some(course_id), Some(tee_id)) => {
            if !rounds::course_tee_matches(
                &state.pool,
                course_id,
                tee_id,
                &input.course_name,
                &input.tee_name,
            )
            .await?
            {
                return Err(ApiError::BadRequest(
                    "course and tee identifiers must match their names".to_owned(),
                ));
            }
        }
        _ => {
            return Err(ApiError::BadRequest(
                "course_id and tee_id must be provided together".to_owned(),
            ));
        }
    }
    Ok(())
}
