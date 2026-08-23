use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State, rejection::BytesRejection},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use chrono::{DateTime, NaiveTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    api::{
        auth::{AuthenticatedSession, MutationSession},
        authorization::map_authorization_error,
    },
    domain::round_pairings::{
        self, FlightCommand, LegacyConversionCommand, PairingMemberCommand, ReplacementCommand,
        TeamCommand,
    },
    error::ApiError,
    repositories::round_pairings::{self as pairing_repository, RoundPairingsError},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplacementRequest {
    expected_round_updated_at: DateTime<Utc>,
    teams: Vec<TeamRequest>,
    flights: Vec<FlightRequest>,
    legacy_conversions: Vec<LegacyConversionRequest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TeamRequest {
    id: Uuid,
    name: String,
    members: Vec<MemberRequest>,
    schedule_flight_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlightRequest {
    id: Uuid,
    name: String,
    starting_hole: Option<i16>,
    tee_time: Option<NaiveTime>,
    members: Vec<MemberRequest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberRequest {
    player_id: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyConversionRequest {
    team_id: Uuid,
    flight_id: Uuid,
}

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> Result<Response, PairingsApiError> {
    let model =
        pairing_repository::get_for_member(&state.pool, authenticated.principal.user_id, round_id)
            .await?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(model)).into_response())
}

pub async fn put(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Result<Response, PairingsApiError> {
    let current_updated_at =
        pairing_repository::preflight_admin(&state.pool, authenticated.principal.user_id, round_id)
            .await?;
    require_json(&headers)?;
    let body = body.map_err(|rejection| {
        if rejection.into_response().status() == StatusCode::PAYLOAD_TOO_LARGE {
            PairingsApiError::PayloadTooLarge
        } else {
            PairingsApiError::Malformed
        }
    })?;
    let request: ReplacementRequest =
        serde_json::from_slice(&body).map_err(|_| PairingsApiError::Malformed)?;
    if request.expected_round_updated_at != current_updated_at {
        return Err(PairingsApiError::Repository(RoundPairingsError::Stale));
    }
    let command = request.to_command();
    round_pairings::validate(&command)
        .map_err(|error| PairingsApiError::Validation(error.to_string()))?;
    let model = pairing_repository::replace(
        &state.pool,
        authenticated.principal.session_id,
        round_id,
        request.expected_round_updated_at,
        &command,
    )
    .await?;
    state.notify("round", round_id);
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(model)).into_response())
}

impl ReplacementRequest {
    fn to_command(&self) -> ReplacementCommand {
        ReplacementCommand {
            teams: self
                .teams
                .iter()
                .map(|team| TeamCommand {
                    id: team.id,
                    name: team.name.clone(),
                    members: members(&team.members),
                    schedule_flight_id: team.schedule_flight_id,
                })
                .collect(),
            flights: self
                .flights
                .iter()
                .map(|flight| FlightCommand {
                    id: flight.id,
                    name: flight.name.clone(),
                    starting_hole: flight.starting_hole,
                    tee_time: flight.tee_time,
                    members: members(&flight.members),
                })
                .collect(),
            legacy_conversions: self
                .legacy_conversions
                .iter()
                .map(|mapping| LegacyConversionCommand {
                    team_id: mapping.team_id,
                    flight_id: mapping.flight_id,
                })
                .collect(),
        }
    }
}

fn members(input: &[MemberRequest]) -> Vec<PairingMemberCommand> {
    input
        .iter()
        .map(|member| PairingMemberCommand {
            player_id: member.player_id,
        })
        .collect()
}

fn require_json(headers: &HeaderMap) -> Result<(), PairingsApiError> {
    let is_json = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if is_json {
        Ok(())
    } else {
        Err(PairingsApiError::UnsupportedMediaType)
    }
}

pub enum PairingsApiError {
    Malformed,
    PayloadTooLarge,
    UnsupportedMediaType,
    Validation(String),
    Repository(RoundPairingsError),
}

impl From<RoundPairingsError> for PairingsApiError {
    fn from(error: RoundPairingsError) -> Self {
        Self::Repository(error)
    }
}

impl IntoResponse for PairingsApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Malformed => ApiError::BadRequest("request must contain an expected timestamp and complete team, flight, and legacy conversion arrays".into()).into_response(),
            Self::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, Json(json!({"error":{"code":"payload_too_large","message":"request body is too large"}}))).into_response(),
            Self::UnsupportedMediaType => ApiError::BadRequest("content-type must be application/json".into()).into_response(),
            Self::Validation(message) => ApiError::BadRequest(message).into_response(),
            Self::Repository(RoundPairingsError::Authorization(error)) => map_authorization_error(error).into_response(),
            Self::Repository(RoundPairingsError::NotDraft) => conflict("round_not_draft", "round must be draft"),
            Self::Repository(RoundPairingsError::Stale) => conflict("round_pairings_stale", "round pairings have changed"),
            Self::Repository(RoundPairingsError::IdentityConflict) => conflict("pairing_identity_conflict", "a team or flight identifier conflicts with stored data"),
            Self::Repository(RoundPairingsError::InvalidRoster) => conflict("invalid_pairing_roster", "submitted members must be active eligible entrants for this format"),
            Self::Repository(RoundPairingsError::LegacyMappingRequired) => conflict("legacy_mapping_required", "every legacy individual group must be mapped exactly once"),
            Self::Repository(RoundPairingsError::InvalidLegacyConversion) => conflict("invalid_legacy_conversion", "legacy conversion must preserve the stored group exactly"),
            Self::Repository(RoundPairingsError::InvalidScheduleTransfer) => conflict("invalid_schedule_transfer", "scheduled scramble teams require one explicit identical flight"),
            Self::Repository(RoundPairingsError::ReferencedTeam) => conflict("team_is_referenced", "a referenced shared-result team cannot be removed"),
            Self::Repository(RoundPairingsError::Database(error)) => ApiError::Database(error).into_response(),
        }
    }
}

fn conflict(code: &'static str, message: &'static str) -> Response {
    (
        StatusCode::CONFLICT,
        Json(json!({"error":{"code":code,"message":message}})),
    )
        .into_response()
}

pub(super) async fn private_no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    response
}
