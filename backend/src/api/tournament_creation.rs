use crate::{
    AppState,
    api::{auth::MutationSession, authorization::map_authorization_error},
    domain::{
        models::ScoringFormat,
        tournament_plan::{self, RoundInput, TournamentPlanInput},
    },
    error::{ApiError, ApiResult},
    rate_limit::RateLimitRoute,
    repositories::tournament_creation::{self, CreationError},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header::CACHE_CONTROL},
    middleware,
    response::Response,
    routing::post,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tournaments", post(create))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("private, no-store"),
                );
                response
            },
        ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    request_id: Uuid,
    tournament: TournamentRequest,
    rounds: Vec<RoundRequest>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TournamentRequest {
    name: String,
    description: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
    counted_rounds: i16,
    mandatory_round_number: Option<i16>,
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
struct Receipt {
    request_id: Uuid,
    tournament_id: Uuid,
    created: bool,
}

async fn create(
    State(state): State<Arc<AppState>>,
    MutationSession(session): MutationSession,
    headers: HeaderMap,
    input: Result<Json<Request>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<Receipt>)> {
    state.rate_limiter.check(
        RateLimitRoute::TournamentCreation,
        state.proxy_trust.client_identity(&headers),
        session.principal.user_id.as_bytes(),
    )?;
    let Json(input) = input.map_err(|_| {
        ApiError::BadRequest("request must contain a request_id, tournament and round plan".into())
    })?;
    if input.request_id.is_nil() {
        return Err(ApiError::BadRequest("request_id must not be nil".into()));
    }
    let plan = tournament_plan::normalize(TournamentPlanInput {
        tournament_name: input.tournament.name,
        description: input.tournament.description,
        start_date: input.tournament.start_date,
        end_date: input.tournament.end_date,
        counted_rounds: input.tournament.counted_rounds,
        mandatory_round_number: input.tournament.mandatory_round_number,
        rounds: input
            .rounds
            .into_iter()
            .map(|r| RoundInput {
                round_number: r.round_number,
                name: r.name,
                round_date: r.round_date,
                scoring_format: r.scoring_format,
            })
            .collect(),
    })
    .map_err(|e| ApiError::BadRequest(e.into()))?;
    let result = tournament_creation::create(
        &state.pool,
        session.principal.session_id,
        input.request_id,
        &plan,
    )
    .await
    .map_err(|e| match e {
        CreationError::Authorization(e) => map_authorization_error(e),
        CreationError::PastEndDate => {
            ApiError::BadRequest("tournament.end_date must not be in the past".into())
        }
        CreationError::ChangedRequest => ApiError::DomainConflict {
            code: "tournament_creation_key_reused",
            message: "retry the original plan or use a new creation request",
        },
        CreationError::Database(e) => ApiError::Database(e),
    })?;
    if result.created {
        state.notify("tournament", result.tournament_id, result.tournament_id);
    }
    Ok((
        if result.created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(Receipt {
            request_id: input.request_id,
            tournament_id: result.tournament_id,
            created: result.created,
        }),
    ))
}
