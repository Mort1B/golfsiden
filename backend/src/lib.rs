pub mod api;
pub mod auth;
pub mod config;
pub mod course_provider;
pub mod domain;
pub mod error;
pub mod repositories;

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub live_events: broadcast::Sender<LiveEvent>,
    pub auth: auth::AuthConfig,
    pub course_provider: course_provider::CourseProviderClient,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct LiveEvent {
    pub resource: &'static str,
    pub id: uuid::Uuid,
}

impl AppState {
    pub fn new(pool: PgPool) -> Arc<Self> {
        Self::with_auth(pool, auth::AuthConfig::local())
    }

    pub fn with_auth(pool: PgPool, auth: auth::AuthConfig) -> Arc<Self> {
        Self::with_auth_and_course_provider(
            pool,
            auth,
            course_provider::CourseProviderClient::disabled(),
        )
    }

    pub fn with_auth_and_course_provider(
        pool: PgPool,
        auth: auth::AuthConfig,
        course_provider: course_provider::CourseProviderClient,
    ) -> Arc<Self> {
        let (live_events, _) = broadcast::channel(128);
        Arc::new(Self {
            pool,
            live_events,
            auth,
            course_provider,
        })
    }

    pub fn notify(&self, resource: &'static str, id: uuid::Uuid) {
        let _ = self.live_events.send(LiveEvent { resource, id });
    }
}
