use super::Error;
use crate::{
    AppState,
    api::auth::AuthenticatedSession,
    error::ApiError,
    repositories::fantasy::results::{
        self, GolferBreakdown, ManagerBreakdown, Results, RoundResult,
    },
};
use axum::{
    Json,
    extract::{Path, State, rejection::PathRejection},
};
use std::sync::Arc;
use uuid::Uuid;
pub(super) async fn overall(
    State(s): State<Arc<AppState>>,
    path: Result<Path<Uuid>, PathRejection>,
    auth: AuthenticatedSession,
) -> Result<Json<Results>, Error> {
    let t = parse_path(path)?;
    let result = results::overall(&s.pool, auth.principal.session_id, t).await?;
    if result.finalized {
        s.notify("tournament", t, t);
    }
    Ok(Json(result))
}
pub(super) async fn round(
    State(s): State<Arc<AppState>>,
    path: Result<Path<(Uuid, Uuid)>, PathRejection>,
    auth: AuthenticatedSession,
) -> Result<Json<RoundResult>, Error> {
    let (t, r) = parse_path(path)?;
    let result = results::round_results(&s.pool, auth.principal.session_id, t, r).await?;
    if result.finalized {
        s.notify("tournament", t, t);
    }
    Ok(Json(result))
}
pub(super) async fn golfer(
    State(s): State<Arc<AppState>>,
    path: Result<Path<(Uuid, Uuid)>, PathRejection>,
    auth: AuthenticatedSession,
) -> Result<Json<GolferBreakdown>, Error> {
    let (t, p) = parse_path(path)?;
    let result = results::golfer_breakdown(&s.pool, auth.principal.session_id, t, p).await?;
    if result.finalized {
        s.notify("tournament", t, t);
    }
    Ok(Json(result))
}
pub(super) async fn manager(
    State(s): State<Arc<AppState>>,
    path: Result<Path<(Uuid, Uuid)>, PathRejection>,
    auth: AuthenticatedSession,
) -> Result<Json<ManagerBreakdown>, Error> {
    let (t, u) = parse_path(path)?;
    let result = results::manager_breakdown(&s.pool, auth.principal.session_id, t, u).await?;
    if result.finalized {
        s.notify("tournament", t, t);
    }
    Ok(Json(result))
}

fn parse_path<T>(path: Result<Path<T>, PathRejection>) -> Result<T, Error> {
    path.map(|Path(value)| value)
        .map_err(|_| ApiError::BadRequest("invalid Fantasy result path".into()).into())
}
