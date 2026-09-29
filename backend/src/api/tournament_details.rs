use crate::{
    AppState,
    api::{auth::MutationSession, authorization::map_authorization_error},
    domain::tournament_details::TournamentDetails,
    error::{ApiError, ApiResult},
    repositories::tournaments::details::{self, DetailsError},
};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::header::CACHE_CONTROL,
    response::IntoResponse,
    routing::patch,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/tournaments/{tournament_id}/details", patch(update))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateDetails {
    name: String,
    description: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
    expected_tournament_updated_at: DateTime<Utc>,
}

async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    MutationSession(session): MutationSession,
    input: Result<Json<UpdateDetails>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("request must contain name, description, start_date, end_date and expected_tournament_updated_at only".into()))?;
    let result = details::update(
        &state.pool,
        session.principal.session_id,
        id,
        input.expected_tournament_updated_at,
        TournamentDetails {
            name: input.name,
            description: input.description,
            start_date: input.start_date,
            end_date: input.end_date,
        },
    )
    .await
    .map_err(map_error)?;
    if result.changed {
        state.notify("tournament", id, id);
    }
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(result.tournament),
    ))
}

fn map_error(error: DetailsError) -> ApiError {
    match error {
        DetailsError::Invalid(message) => ApiError::BadRequest(message.into()),
        DetailsError::Locked => ApiError::DomainConflict {
            code: "tournament_details_locked",
            message: "only draft tournaments can change details",
        },
        DetailsError::Stale => ApiError::DomainConflict {
            code: "tournament_details_stale",
            message: "tournament changed; refresh and try again",
        },
        DetailsError::RoundDates => ApiError::DomainConflict {
            code: "tournament_details_round_dates",
            message: "tournament dates must contain every configured round date",
        },
        DetailsError::Authorization(error) => map_authorization_error(error),
        DetailsError::Database(error)
            if error
                .as_database_error()
                .and_then(|error| error.constraint())
                == Some("tournament_details_session") =>
        {
            ApiError::Unauthenticated
        }
        DetailsError::Database(error) => ApiError::Database(error),
    }
}
