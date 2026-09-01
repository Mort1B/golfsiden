use std::{convert::Infallible, sync::Arc};

use axum::{
    extract::{Path, State},
    http::header::CACHE_CONTROL,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use uuid::Uuid;

use crate::{
    AppState,
    api::{auth::AuthenticatedSession, authorization::map_authorization_error},
    error::ApiResult,
    repositories::live,
};

const INVALIDATION_DATA: &str = "invalidate";

pub async fn events(
    State(state): State<Arc<AppState>>,
    Path(tournament_id): Path<Uuid>,
    authenticated: AuthenticatedSession,
) -> ApiResult<Response> {
    let session_id = authenticated.principal.session_id;
    live::authorize(&state.pool, session_id, tournament_id)
        .await
        .map_err(map_authorization_error)?;
    let mut receiver = state.live_events.subscribe();
    let pool = state.pool.clone();
    let stream = async_stream::stream! {
        loop {
            match receiver.recv().await {
                Ok(event) if event.tournament_id != tournament_id => continue,
                Ok(event) => {
                    if live::authorize(&pool, session_id, tournament_id).await.is_err() {
                        break;
                    }
                    yield Ok::<Event, Infallible>(
                        Event::default()
                            .event(event.resource)
                            .data(INVALIDATION_DATA),
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => break,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Ok((
        [(CACHE_CONTROL, "private, no-store")],
        Sse::new(stream).keep_alive(KeepAlive::default()),
    )
        .into_response())
}
