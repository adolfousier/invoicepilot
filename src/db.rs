use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, Postgres, Pool, Row};
use std::env;

pub type DbPool = Pool<Postgres>;

pub async fn init_pool() -> Result<DbPool> {
    // Load .env file from multiple locations (same as config does)
    if dotenvy::dotenv().is_err() {
        if dotenvy::from_path("docker/.env").is_err() {
            dotenvy::from_path("../.env").ok();
        }
    }

    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            return Err(anyhow::anyhow!("DATABASE_URL not configured"));
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    // Create table if it doesn't exist
    create_tables(&pool).await?;

    Ok(pool)
}

async fn create_tables(pool: &DbPool) -> Result<()> {
    // Create table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS activity_logs (
            id SERIAL PRIMARY KEY,
            message TEXT NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await
    .context("Failed to create activity_logs table")?;

    // Create index separately
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_activity_logs_created_at ON activity_logs(created_at DESC)
        "#
    )
    .execute(pool)
    .await
    .context("Failed to create index on activity_logs")?;

    Ok(())
}

pub async fn save_log(pool: &DbPool, message: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO activity_logs (message, created_at)
        VALUES ($1, $2)
        "#
    )
    .bind(message)
    .bind(Utc::now())
    .execute(pool)
    .await
    .context("Failed to insert log message")?;

    Ok(())
}

pub async fn load_logs(pool: &DbPool) -> Result<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT message
        FROM activity_logs
        ORDER BY created_at ASC
        LIMIT 1000
        "#
    )
    .fetch_all(pool)
    .await
    .context("Failed to load logs from database")?;

    let messages = rows
        .iter()
        .map(|row| row.get::<String, _>("message"))
        .collect();

    Ok(messages)
}

