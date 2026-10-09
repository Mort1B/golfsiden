mod handlers;
mod results;
use crate::{AppState, error::ApiError, repositories::fantasy::Error as RepositoryError};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, header::CACHE_CONTROL},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use std::sync::Arc;
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/tournaments/{t}/fantasy/results",
            get(results::overall),
        )
        .route(
            "/api/tournaments/{t}/fantasy/rounds/{r}/results",
            get(results::round),
        )
        .route(
            "/api/tournaments/{t}/fantasy/results/golfers/{p}",
            get(results::golfer),
        )
        .route(
            "/api/tournaments/{t}/fantasy/results/managers/{u}",
            get(results::manager),
        )
        .route(
            "/api/tournaments/{t}/fantasy",
            get(handlers::game).put(handlers::configure),
        )
        .route("/api/tournaments/{t}/fantasy/entry", post(handlers::enter))
        .route(
            "/api/tournaments/{t}/fantasy/rounds/{r}",
            get(handlers::round),
        )
        .route(
            "/api/tournaments/{t}/fantasy/rounds/{r}/deadline",
            put(handlers::deadline),
        )
        .route(
            "/api/tournaments/{t}/fantasy/rounds/{r}/lineup",
            put(handlers::save),
        )
        .route(
            "/api/tournaments/{t}/fantasy/rounds/{r}/owners/{kind}/{owner}",
            get(handlers::source).post(handlers::dispose),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::map_response(no_store))
}
async fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
#[derive(Debug)]
pub(super) struct Error(ApiError);
impl From<ApiError> for Error {
    fn from(e: ApiError) -> Self {
        Self(e)
    }
}
impl From<RepositoryError> for Error {
    fn from(e: RepositoryError) -> Self {
        Self(match e {
            RepositoryError::Authorization(e) => super::authorization::map_authorization_error(e),
            RepositoryError::Unavailable => ApiError::DomainConflict {
                code: "fantasy_unavailable",
                message: "Fantasy is unavailable or entry is required",
            },
            RepositoryError::Closed => ApiError::DomainConflict {
                code: "fantasy_closed",
                message: "the Fantasy selection window is closed",
            },
            RepositoryError::Invalid => {
                ApiError::BadRequest("invalid Fantasy request, lineup or owner".into())
            }
            RepositoryError::RevisionConflict => ApiError::DomainConflict {
                code: "fantasy_revision_conflict",
                message: "this request was not accepted; the lineup revision changed",
            },
            RepositoryError::Conflict => ApiError::DomainConflict {
                code: "fantasy_conflict",
                message: "Fantasy state changed; refresh and retry",
            },
            RepositoryError::Database(e) => ApiError::Database(e),
        })
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}
