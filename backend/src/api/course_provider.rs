use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    api::{auth::AuthenticatedSession, authorization::map_authorization_error},
    course_catalog::{CourseCatalogError, ProviderCourseReadiness, provider_course_readiness},
    course_provider::{CourseDetail, CourseProviderError, normalize_course_id},
    error::ApiError,
    repositories::tournament_authorization,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/tournaments/{tournament_id}/course-provider/courses/{provider_course_id}",
        get(course),
    )
}

async fn course(
    State(state): State<Arc<AppState>>,
    Path((tournament_id, provider_course_id)): Path<(Uuid, String)>,
    authenticated: AuthenticatedSession,
) -> Result<impl IntoResponse, CourseProviderApiError> {
    authorize(&state, &authenticated, tournament_id).await?;
    let course = fetch_course(&state, &provider_course_id).await?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(course)))
}

pub(crate) async fn fetch_course(
    state: &AppState,
    provider_course_id: &str,
) -> Result<CourseDetail, CourseProviderApiError> {
    let provider_course_id = normalize_course_id(provider_course_id)
        .map_err(|message| CourseProviderApiError::Api(ApiError::BadRequest(message.to_owned())))?;
    match provider_course_readiness(&provider_course_id)? {
        ProviderCourseReadiness::Usable => {}
        ProviderCourseReadiness::Incomplete => {
            return Err(CourseProviderApiError::CatalogIncomplete);
        }
        ProviderCourseReadiness::Unknown => return Err(CourseProviderApiError::CatalogUnknown),
    }
    state
        .course_provider
        .course(&provider_course_id)
        .await
        .map_err(CourseProviderApiError::Provider)
}

async fn authorize(
    state: &AppState,
    authenticated: &AuthenticatedSession,
    tournament_id: Uuid,
) -> Result<(), CourseProviderApiError> {
    tournament_authorization::require_tournament_admin_read(
        &state.pool,
        authenticated.principal.user_id,
        tournament_id,
    )
    .await
    .map_err(map_authorization_error)?;
    Ok(())
}

pub(crate) enum CourseProviderApiError {
    Api(ApiError),
    CatalogIncomplete,
    CatalogUnknown,
    Provider(CourseProviderError),
}

impl From<ApiError> for CourseProviderApiError {
    fn from(value: ApiError) -> Self {
        Self::Api(value)
    }
}

impl From<CourseProviderError> for CourseProviderApiError {
    fn from(value: CourseProviderError) -> Self {
        Self::Provider(value)
    }
}

impl From<CourseCatalogError> for CourseProviderApiError {
    fn from(_: CourseCatalogError) -> Self {
        Self::Api(ApiError::Internal)
    }
}

impl IntoResponse for CourseProviderApiError {
    fn into_response(self) -> Response {
        let response = match self {
            Self::Api(error) => error.into_response(),
            Self::CatalogIncomplete => (
                StatusCode::CONFLICT,
                Json(json!({"error": {
                    "code": "course_catalog_incomplete",
                    "message": "course provider detail is unavailable because the catalog entry is incomplete"
                }})),
            )
                .into_response(),
            Self::CatalogUnknown => (
                StatusCode::NOT_FOUND,
                Json(json!({"error": {
                    "code": "course_catalog_not_found",
                    "message": "course is not in the local catalog"
                }})),
            )
                .into_response(),
            Self::Provider(error) => {
                let (status, code, message) = match error {
                    CourseProviderError::NotFound => (
                        StatusCode::NOT_FOUND,
                        "course_provider_not_found",
                        "course was not found",
                    ),
                    CourseProviderError::Exhausted => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "course_provider_exhausted",
                        "course provider request allowance is exhausted",
                    ),
                    CourseProviderError::Saturated => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "course_provider_busy",
                        "course provider is busy; retry later",
                    ),
                    CourseProviderError::Timeout => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "course_provider_timeout",
                        "course provider timed out; retry later",
                    ),
                    CourseProviderError::InvalidResponse => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "course_provider_invalid_response",
                        "course provider returned an unusable response",
                    ),
                    CourseProviderError::Unavailable | CourseProviderError::Upstream => (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "course_provider_unavailable",
                        "course provider is unavailable",
                    ),
                };
                (
                    status,
                    Json(json!({"error": {"code": code, "message": message}})),
                )
                    .into_response()
            }
        };
        let mut response = response;
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
        response
    }
}
