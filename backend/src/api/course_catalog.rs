use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderValue, header::CACHE_CONTROL},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    api::{auth::AuthenticatedSession, authorization::map_authorization_error},
    course_catalog::{self, CourseCatalogItem},
    error::ApiError,
    repositories::tournament_authorization,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/tournaments/{tournament_id}/course-catalog", get(list))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogQuery {
    q: Option<String>,
}

#[derive(Serialize)]
struct CatalogResponse {
    courses: Vec<CourseCatalogItem>,
}

async fn list(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
    input: Result<Query<CatalogQuery>, QueryRejection>,
) -> Result<impl IntoResponse, CatalogApiError> {
    tournament_authorization::require_tournament_admin_read(
        &state.pool,
        authenticated.principal.user_id,
        tournament_id,
    )
    .await
    .map_err(map_authorization_error)?;
    let Query(input) = input.map_err(|_| {
        CatalogApiError(ApiError::BadRequest(
            "query must contain only an optional text q".to_owned(),
        ))
    })?;
    let courses = course_catalog::search(input.q.as_deref()).map_err(|error| match error {
        course_catalog::CourseCatalogError::InvalidQuery => ApiError::BadRequest(error.to_string()),
        course_catalog::CourseCatalogError::InvalidCatalog => ApiError::Internal,
    })?;
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Json(CatalogResponse { courses }),
    ))
}

struct CatalogApiError(ApiError);

impl From<ApiError> for CatalogApiError {
    fn from(value: ApiError) -> Self {
        Self(value)
    }
}

impl IntoResponse for CatalogApiError {
    fn into_response(self) -> Response {
        let mut response = self.0.into_response();
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
        response
    }
}
