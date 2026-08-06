use sqlx::postgres::PgPoolOptions;

use golf_api::auth::hash_password;

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
    let password_hash = hash_password(b"golf-dev-2026")
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    sqlx::query(
        "UPDATE users SET password_hash = $1
         WHERE id = '00000000-0000-0000-0000-000000000001'
            OR player_id BETWEEN '00000000-0000-0000-0000-000000001001'
                             AND '00000000-0000-0000-0000-000000001008'",
    )
    .bind(password_hash)
    .execute(&pool)
    .await?;
    println!("Seeded Guttas Golf 2026 with eight players and five rounds.");
    Ok(())
}
