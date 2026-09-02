use std::net::SocketAddr;

use golf_api::{
    AppState, api,
    config::{AppEnvironment, Config},
    course_provider::CourseProviderClient,
    rate_limit::RateLimiter,
    schema,
};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "golf_api=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;

    if config.run_migrations {
        schema::MIGRATOR.run(&pool).await?;
    }
    schema::check_compatibility(&pool).await?;
    if let Some(expected_user) = &config.expected_database_user {
        schema::check_runtime_authority(&pool, expected_user).await?;
    }

    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(address).await?;
    info!(%address, "golf API listening");

    let course_provider = match config.course_provider.api_key {
        Some(api_key) => CourseProviderClient::configured_with_daily_limit(
            api_key,
            config.course_provider.daily_limit,
        )?,
        None => CourseProviderClient::disabled(),
    };
    let rate_limiter = match config.environment {
        AppEnvironment::Production => RateLimiter::production(),
        AppEnvironment::Development => RateLimiter::disabled(),
    };
    axum::serve(
        listener,
        api::router(AppState::with_runtime_services_and_proxy(
            pool,
            config.auth,
            course_provider,
            rate_limiter,
            config.proxy_trust,
        )),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler")
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}
