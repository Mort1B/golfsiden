use golf_api::{AppState, api, auth::AuthConfig, config::RecoveryOrigin};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL")?;
    assert!(
        url.starts_with("postgres://golf_review:") && url.ends_with("@127.0.0.1:55443/golf_review")
    );
    let pool = sqlx::PgPool::connect(&url).await?;
    golf_api::schema::MIGRATOR.run(&pool).await?;
    let mut config = AuthConfig::local();
    config.recovery_origin = Some(RecoveryOrigin::parse("http://127.0.0.1:5173", true)?);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Local recovery review API ready");
    axum::serve(listener, api::router(AppState::with_auth(pool, config))).await?;
    Ok(())
}
