use super::Error;
use crate::{
    AppState,
    api::auth::{AuthenticatedSession, MutationSession},
    error::ApiError,
    repositories::fantasy::{
        self, Dispose, Game, OwnerKind, Receipt, RoundView, Save, Source, Window,
    },
};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Configure {
    enabled: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Deadline {
    deadline: Option<DateTime<Utc>>,
}
fn body<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, Error> {
    input
        .map(|Json(i)| i)
        .map_err(|_| ApiError::BadRequest("invalid Fantasy request body".into()).into())
}
pub(super) async fn game(
    State(s): State<Arc<AppState>>,
    Path(t): Path<Uuid>,
    auth: AuthenticatedSession,
) -> Result<Json<Option<Game>>, Error> {
    Ok(Json(
        fantasy::game(&s.pool, auth.principal.session_id, t).await?,
    ))
}
pub(super) async fn configure(
    State(s): State<Arc<AppState>>,
    Path(t): Path<Uuid>,
    MutationSession(auth): MutationSession,
    input: Result<Json<Configure>, JsonRejection>,
) -> Result<Json<Game>, Error> {
    let result =
        fantasy::configure(&s.pool, auth.principal.session_id, t, body(input)?.enabled).await?;
    s.notify("tournament", t, t);
    Ok(Json(result))
}
pub(super) async fn enter(
    State(s): State<Arc<AppState>>,
    Path(t): Path<Uuid>,
    MutationSession(auth): MutationSession,
) -> Result<StatusCode, Error> {
    fantasy::enter(&s.pool, auth.principal.session_id, t).await?;
    s.notify("tournament", t, t);
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn deadline(
    State(s): State<Arc<AppState>>,
    Path((t, r)): Path<(Uuid, Uuid)>,
    MutationSession(auth): MutationSession,
    input: Result<Json<Deadline>, JsonRejection>,
) -> Result<Json<Window>, Error> {
    let result = fantasy::set_deadline(
        &s.pool,
        auth.principal.session_id,
        t,
        r,
        body(input)?.deadline,
    )
    .await?;
    s.notify("round", t, r);
    Ok(Json(result))
}
pub(super) async fn save(
    State(s): State<Arc<AppState>>,
    Path((t, r)): Path<(Uuid, Uuid)>,
    MutationSession(auth): MutationSession,
    input: Result<Json<Save>, JsonRejection>,
) -> Result<Json<Receipt>, Error> {
    let result = fantasy::save(&s.pool, auth.principal.session_id, t, r, &body(input)?).await?;
    s.notify("round", t, r);
    Ok(Json(result))
}
pub(super) async fn round(
    State(s): State<Arc<AppState>>,
    Path((t, r)): Path<(Uuid, Uuid)>,
    auth: AuthenticatedSession,
) -> Result<Json<RoundView>, Error> {
    let view = fantasy::read_round(&s.pool, auth.principal.session_id, t, r).await?;
    if view.finalized {
        s.notify("tournament", t, t);
    }
    Ok(Json(view))
}
pub(super) async fn source(
    State(s): State<Arc<AppState>>,
    Path((t, r, kind, owner)): Path<(Uuid, Uuid, OwnerKind, Uuid)>,
    auth: AuthenticatedSession,
) -> Result<Json<Source>, Error> {
    Ok(Json(
        fantasy::source(&s.pool, auth.principal.session_id, t, r, kind, owner).await?,
    ))
}
pub(super) async fn dispose(
    State(s): State<Arc<AppState>>,
    Path((t, r, kind, owner)): Path<(Uuid, Uuid, OwnerKind, Uuid)>,
    MutationSession(auth): MutationSession,
    input: Result<Json<Dispose>, JsonRejection>,
) -> Result<Json<Source>, Error> {
    let result = fantasy::dispose(
        &s.pool,
        auth.principal.session_id,
        t,
        r,
        kind,
        owner,
        &body(input)?,
    )
    .await?;
    s.notify("round", t, r);
    Ok(Json(result))
}
