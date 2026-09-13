use super::*;
use crate::{
    domain::{
        leaderboards::LeaderboardMetric,
        result_sharing::{PublicResults, ResultShareToken},
    },
    error::ApiResult,
    rate_limit::RateLimitRoute,
    repositories::result_sharing,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReadRequest {
    token: String,
    metric: LeaderboardMetric,
}
#[derive(Serialize)]
pub(super) struct Results {
    grant_id: Uuid,
    expires_at: DateTime<Utc>,
    #[serde(flatten)]
    results: PublicResults,
}
pub(super) async fn read(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<ReadRequest>, JsonRejection>,
) -> ApiResult<Json<Results>> {
    // No session extractor: cookies confer no public projection authority.
    state.rate_limiter.check(
        RateLimitRoute::PublicResults,
        state.proxy_trust.client_identity(&headers),
        id.as_bytes(),
    )?;
    let id = id.parse().map_err(|_| map_error(ShareError::Unavailable))?;
    let Json(input) = input.map_err(input_error)?;
    let token =
        ResultShareToken::parse(input.token).ok_or_else(|| map_error(ShareError::Unavailable))?;
    let read = result_sharing::read(&state.pool, id, &token, input.metric)
        .await
        .map_err(map_error)?;
    Ok(Json(Results {
        grant_id: read.grant_id,
        expires_at: read.expires_at,
        results: read.results,
    }))
}
