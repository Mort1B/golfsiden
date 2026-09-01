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
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    api::{
        auth::MutationSession,
        authorization::map_authorization_error,
        course_provider::{CourseProviderApiError, fetch_course},
    },
    course_provider::{
        TeeCategory as ProviderTeeCategory,
        revision_adapter::{ProviderRevisionError, select_and_validate},
    },
    domain::course_revisions::{
        self, CourseRevisionCommand, CourseRevisionSource, HoleRevisionCommand, TeeCategory,
        TeeRevisionCommand,
    },
    error::ApiError,
    repositories::round_configuration::{self, RoundConfigurationError},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    expected_round_updated_at: DateTime<Utc>,
    selection: Value,
}

#[derive(Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
enum Selection {
    Manual {
        course_name: String,
        location: Option<String>,
        tee: ManualTee,
    },
    GolfCourseApi {
        provider_course_id: String,
        tee: ProviderTeeSelector,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManualTee {
    category: ApiTeeCategory,
    name: String,
    course_rating: f64,
    slope_rating: i16,
    holes: Vec<ManualHole>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManualHole {
    par: i16,
    stroke_index: i16,
    distance: Option<i16>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderTeeSelector {
    category: ApiTeeCategory,
    name: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ApiTeeCategory {
    Female,
    Male,
}

pub async fn put(
    State(state): State<Arc<AppState>>,
    Path(round_id): Path<Uuid>,
    MutationSession(authenticated): MutationSession,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Result<Response, ConfigurationApiError> {
    let preflight =
        round_configuration::preflight(&state.pool, authenticated.principal.user_id, round_id)
            .await?;
    require_json_content_type(&headers)?;
    let body = body.map_err(|rejection| {
        if rejection.into_response().status() == StatusCode::PAYLOAD_TOO_LARGE {
            ConfigurationApiError::PayloadTooLarge
        } else {
            ConfigurationApiError::Malformed
        }
    })?;
    let envelope: Envelope =
        serde_json::from_slice(&body).map_err(|_| ConfigurationApiError::Malformed)?;
    if envelope.expected_round_updated_at != preflight.updated_at {
        return Err(ConfigurationApiError::Repository(
            RoundConfigurationError::Stale,
        ));
    }

    let selection: Selection =
        serde_json::from_value(envelope.selection).map_err(|_| ConfigurationApiError::Malformed)?;
    let revision = match selection {
        Selection::Manual {
            course_name,
            location,
            tee,
        } => course_revisions::validate(manual_command(course_name, location, tee))
            .map_err(|error| ConfigurationApiError::Validation(error.to_string()))?,
        Selection::GolfCourseApi {
            provider_course_id,
            tee,
        } => {
            let detail = fetch_course(&state, &provider_course_id).await?;
            select_and_validate(detail, tee.category.provider(), &tee.name).map_err(|error| {
                match error {
                    ProviderRevisionError::TeeStale => ConfigurationApiError::ProviderTeeStale,
                    ProviderRevisionError::InvalidFacts(_) => {
                        ConfigurationApiError::ProviderInvalid
                    }
                }
            })?
        }
    };

    let round = round_configuration::configure(
        &state.pool,
        authenticated.principal.session_id,
        round_id,
        envelope.expected_round_updated_at,
        &revision,
    )
    .await?;
    state.notify("round", round.tournament_id, round_id);
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(round)).into_response())
}

fn manual_command(
    course_name: String,
    location: Option<String>,
    tee: ManualTee,
) -> CourseRevisionCommand {
    CourseRevisionCommand {
        source: CourseRevisionSource::Manual,
        provider_course_id: None,
        course_name,
        location,
        tee: TeeRevisionCommand {
            category: tee.category.revision(),
            name: tee.name,
            course_rating: tee.course_rating,
            slope_rating: tee.slope_rating,
            holes: tee
                .holes
                .into_iter()
                .map(|hole| HoleRevisionCommand {
                    par: hole.par,
                    stroke_index: hole.stroke_index,
                    distance: hole.distance,
                })
                .collect(),
        },
    }
}

impl ApiTeeCategory {
    fn revision(self) -> TeeCategory {
        match self {
            Self::Female => TeeCategory::Female,
            Self::Male => TeeCategory::Male,
        }
    }

    fn provider(self) -> ProviderTeeCategory {
        match self {
            Self::Female => ProviderTeeCategory::Female,
            Self::Male => ProviderTeeCategory::Male,
        }
    }
}

pub enum ConfigurationApiError {
    Malformed,
    PayloadTooLarge,
    UnsupportedMediaType,
    Validation(String),
    ProviderTeeStale,
    ProviderInvalid,
    Provider(CourseProviderApiError),
    Repository(RoundConfigurationError),
}

impl From<CourseProviderApiError> for ConfigurationApiError {
    fn from(error: CourseProviderApiError) -> Self {
        Self::Provider(error)
    }
}

impl From<RoundConfigurationError> for ConfigurationApiError {
    fn from(error: RoundConfigurationError) -> Self {
        Self::Repository(error)
    }
}

impl IntoResponse for ConfigurationApiError {
    fn into_response(self) -> Response {
        let response = match self {
            Self::Malformed => ApiError::BadRequest(
                "request must contain an expected timestamp and one valid course selection"
                    .to_owned(),
            )
            .into_response(),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(json!({"error": {
                    "code": "payload_too_large",
                    "message": "request body is too large"
                }})),
            )
                .into_response(),
            Self::UnsupportedMediaType => {
                ApiError::BadRequest("content-type must be application/json".to_owned())
                    .into_response()
            }
            Self::Validation(message) => ApiError::BadRequest(message).into_response(),
            Self::Provider(error) => return error.into_response(),
            Self::ProviderInvalid => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": {
                    "code": "course_provider_invalid_response",
                    "message": "course provider returned an unusable response"
                }})),
            )
                .into_response(),
            Self::ProviderTeeStale => conflict(
                "course_provider_tee_stale",
                "the selected provider tee is no longer available",
            ),
            Self::Repository(RoundConfigurationError::NotDraft) => {
                conflict("round_not_draft", "round must be draft")
            }
            Self::Repository(RoundConfigurationError::Stale) => conflict(
                "round_configuration_stale",
                "round configuration has changed",
            ),
            Self::Repository(RoundConfigurationError::Authorization(error)) => {
                map_authorization_error(error).into_response()
            }
            Self::Repository(RoundConfigurationError::Database(error)) => {
                ApiError::Database(error).into_response()
            }
            Self::Repository(RoundConfigurationError::CourseRevision(error)) => match error {
                crate::repositories::course_revisions::CourseRevisionRepositoryError::Database(
                    error,
                ) => ApiError::Database(error).into_response(),
                _ => ApiError::Internal.into_response(),
            },
        };
        private(response)
    }
}

fn require_json_content_type(headers: &HeaderMap) -> Result<(), ConfigurationApiError> {
    let is_json = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if !is_json {
        return Err(ConfigurationApiError::UnsupportedMediaType);
    }
    Ok(())
}

fn conflict(code: &'static str, message: &'static str) -> Response {
    (
        StatusCode::CONFLICT,
        Json(json!({"error": {"code": code, "message": message}})),
    )
        .into_response()
}

fn private(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    response
}
