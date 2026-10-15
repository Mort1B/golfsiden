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
    api::{
        auth::AuthenticatedSession,
        private_read::{map_error as map_read_error, response as private_response},
    },
    course_catalog::{self, CourseCatalogItem},
    error::ApiError,
    repositories::{course_presets, course_reads},
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tournaments/{tournament_id}/course-catalog", get(list))
        .route(
            "/api/tournaments/{tournament_id}/course-presets",
            get(presets),
        )
        .layer(axum::middleware::map_response(private_response))
}

async fn presets(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> Result<impl IntoResponse, CatalogApiError> {
    let courses = course_presets::list_for_session(
        &state.pool,
        authenticated.principal.session_id,
        tournament_id,
    )
    .await
    .map_err(map_read_error)?;
    Ok(([(CACHE_CONTROL, "private, no-store")], Json(courses)))
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
    let courses = course_reads::local_for_session(
        &state.pool,
        authenticated.principal.session_id,
        tournament_id,
        || -> Result<_, CatalogApiError> {
            let Query(input) = input.as_ref().map_err(|_| {
                ApiError::BadRequest("query must contain only an optional text q".to_owned())
            })?;
            Ok(
                course_catalog::search(input.q.as_deref()).map_err(|error| match error {
                    course_catalog::CourseCatalogError::InvalidQuery => {
                        ApiError::BadRequest(error.to_string())
                    }
                    course_catalog::CourseCatalogError::InvalidCatalog => ApiError::Internal,
                })?,
            )
        },
    )
    .await
    .map_err(map_read_error)??;
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
