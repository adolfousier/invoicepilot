//! Database layer for persisting activity logs.
//!
//! Backend selected at compile time:
//! - Default (`cargo install invoicepilot`) → SQLite at `~/.config/invoice-pilot/activity.db`
//! - `--features postgres` → PostgreSQL via `DATABASE_URL`

use anyhow::{Context, Result};
use chrono::Utc;

// ── PostgreSQL backend ──────────────────────────────────────────────────────

#[cfg(feature = "postgres")]
pub type DbPool = sqlx::Pool<sqlx::Postgres>;

#[cfg(feature = "postgres")]
pub async fn init_pool() -> Result<DbPool> {
    use sqlx::postgres::PgPoolOptions;
    use std::env;

    if dotenvy::dotenv().is_err()
        && dotenvy::from_path("docker/.env").is_err() {
            dotenvy::from_path("../.env").ok();
        }

    let database_url = env::var("DATABASE_URL")
        .context("DATABASE_URL not configured")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS activity_logs (
            id SERIAL PRIMARY KEY,
            message TEXT NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        )"
    )
    .execute(&pool)
    .await
    .context("Failed to create activity_logs table")?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_activity_logs_created_at ON activity_logs(created_at DESC)"
    )
    .execute(&pool)
    .await
    .context("Failed to create index on activity_logs")?;

    Ok(pool)
}

#[cfg(feature = "postgres")]
pub async fn save_log(pool: &DbPool, message: &str) -> Result<()> {
    sqlx::query("INSERT INTO activity_logs (message, created_at) VALUES ($1, $2)")
        .bind(message)
        .bind(Utc::now())
        .execute(pool)
        .await
        .context("Failed to insert log message")?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub async fn load_logs(pool: &DbPool) -> Result<Vec<String>> {
    use sqlx::Row as _;

    let rows = sqlx::query("SELECT message FROM activity_logs ORDER BY created_at ASC LIMIT 1000")
        .fetch_all(pool)
        .await
        .context("Failed to load logs from database")?;

    Ok(rows.iter().map(|row| row.get::<String, _>("message")).collect())
}

// ── SQLite backend (default for cargo install) ──────────────────────────────

#[cfg(not(feature = "postgres"))]
pub type DbPool = sqlx::Pool<sqlx::Sqlite>;

#[cfg(not(feature = "postgres"))]
pub async fn init_pool() -> Result<DbPool> {
    use sqlx::sqlite::SqlitePoolOptions;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("invoice-pilot");

    std::fs::create_dir_all(&config_dir)
        .context("Failed to create config directory for SQLite")?;

    let db_path = config_dir.join("activity.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .context("Failed to open SQLite database")?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS activity_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now'))
        )"
    )
    .execute(&pool)
    .await
    .context("Failed to create activity_logs table")?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_activity_logs_created_at ON activity_logs(created_at DESC)"
    )
    .execute(&pool)
    .await
    .context("Failed to create index on activity_logs")?;

    Ok(pool)
}

#[cfg(not(feature = "postgres"))]
pub async fn save_log(pool: &DbPool, message: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO activity_logs (message, created_at) VALUES (?1, ?2)")
        .bind(message)
        .bind(now)
        .execute(pool)
        .await
        .context("Failed to insert log message")?;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
pub async fn load_logs(pool: &DbPool) -> Result<Vec<String>> {
    use sqlx::Row as _;

    let rows = sqlx::query("SELECT message FROM activity_logs ORDER BY created_at ASC LIMIT 1000")
        .fetch_all(pool)
        .await
        .context("Failed to load logs from database")?;

    Ok(rows.iter().map(|row| row.get::<String, _>("message")).collect())
}
