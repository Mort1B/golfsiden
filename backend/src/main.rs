use std::net::SocketAddr;

use golf_api::{AppState, api, config::Config, course_provider::CourseProviderClient};
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
        sqlx::migrate!("../migrations").run(&pool).await?;
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
    axum::serve(
        listener,
        api::router(AppState::with_auth_and_course_provider(
            pool,
            config.auth,
            course_provider,
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
