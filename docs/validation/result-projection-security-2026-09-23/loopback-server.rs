#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("DATABASE_URL")?;
    assert!(
        url.starts_with("postgres://golf_review:") && url.ends_with("@127.0.0.1:55443/golf_review")
    );
    let pool = sqlx::PgPool::connect(&url).await?;
    golf_api::schema::MIGRATOR.run(&pool).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Local results review API ready");
    axum::serve(
        listener,
        golf_api::api::router(golf_api::AppState::new(pool)),
    )
    .await?;
    Ok(())
}
