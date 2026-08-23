use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    api::{auth::AuthenticatedSession, authorization::map_authorization_error},
    course_provider::{
        CourseDetail, CourseProviderError, CourseSearchResult, normalize_course_id,
        validate_search_query,
    },
    error::ApiError,
    repositories::tournament_authorization,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/tournaments/{tournament_id}/course-provider/search",
            get(search),
        )
        .route(
            "/api/tournaments/{tournament_id}/course-provider/courses/{provider_course_id}",
            get(course),
        )
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_fuzzy_match")]
    fuzzy_match: bool,
}

fn default_fuzzy_match() -> bool {
    true
}

#[derive(Serialize)]
struct SearchResponse {
    courses: Vec<CourseSearchResult>,
}

async fn search(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
    input: Result<Query<SearchQuery>, QueryRejection>,
) -> Result<impl IntoResponse, CourseProviderApiError> {
    authorize(&state, &authenticated, tournament_id).await?;
    let Query(input) = input.map_err(|_| {
        CourseProviderApiError::Api(ApiError::BadRequest(
            "query must contain q and optional boolean fuzzy_match".to_owned(),
        ))
    })?;
    let query = validate_search_query(&input.q)
        .map_err(|message| CourseProviderApiError::Api(ApiError::BadRequest(message.to_owned())))?;
    let courses = state
        .course_provider
        .search(query, input.fuzzy_match)
        .await?;
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(SearchResponse { courses }),
    ))
}

async fn course(
    State(state): State<Arc<AppState>>,
    Path((tournament_id, provider_course_id)): Path<(Uuid, String)>,
    authenticated: AuthenticatedSession,
) -> Result<impl IntoResponse, CourseProviderApiError> {
    authorize(&state, &authenticated, tournament_id).await?;
    let provider_course_id = normalize_course_id(&provider_course_id)
        .map_err(|message| CourseProviderApiError::Api(ApiError::BadRequest(message.to_owned())))?;
    let course: CourseDetail = state.course_provider.course(&provider_course_id).await?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(course)))
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

enum CourseProviderApiError {
    Api(ApiError),
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

impl IntoResponse for CourseProviderApiError {
    fn into_response(self) -> Response {
        let response = match self {
            Self::Api(error) => return error.into_response(),
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
