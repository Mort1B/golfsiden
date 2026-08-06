use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    sqlx::raw_sql(include_str!("../../seed.sql"))
        .execute(&pool)
        .await?;
    println!("Seeded Guttas Golf 2026 with eight players and five rounds.");
    Ok(())
}
