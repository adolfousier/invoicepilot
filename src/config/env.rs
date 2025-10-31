use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate};
use log::info;
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // Gmail Account credentials
    pub gmail_client_id: String,
    pub gmail_client_secret: String,

    // Drive Account credentials
    pub drive_client_id: String,
    pub drive_client_secret: String,
    pub drive_folder_path: String,

    // Scheduling (only required for scheduled mode)
    pub fetch_invoices_day: Option<u8>,

    // Keywords to search for in emails
    pub target_keywords: Vec<String>,

    // Date range for filtering emails
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,

    // Debug logging
    pub debug_logs_enabled: bool,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file from multiple possible locations
        // Priority: 1. Current directory, 2. docker/.env, 3. Parent directory
        if dotenvy::dotenv().is_err() {
            if dotenvy::from_path("docker/.env").is_err() {
                dotenvy::from_path("../.env").ok();
            }
        }

        // Parse date range
        let (start_date, end_date) = Self::parse_date_range()?;

        let config = Config {
            gmail_client_id: env::var("GOOGLE_GMAIL_CLIENT_ID")
                .context("GOOGLE_GMAIL_CLIENT_ID not set in .env")?,
            gmail_client_secret: env::var("GOOGLE_GMAIL_CLIENT_SECRET")
                .context("GOOGLE_GMAIL_CLIENT_SECRET not set in .env")?,
            drive_client_id: env::var("GOOGLE_DRIVE_CLIENT_ID")
                .context("GOOGLE_DRIVE_CLIENT_ID not set in .env")?,
            drive_client_secret: env::var("GOOGLE_DRIVE_CLIENT_SECRET")
                .context("GOOGLE_DRIVE_CLIENT_SECRET not set in .env")?,
            drive_folder_path: env::var("GOOGLE_DRIVE_FOLDER_LOCATION")
                .context("GOOGLE_DRIVE_FOLDER_LOCATION not set in .env")?,
            fetch_invoices_day: env::var("FETCH_INVOICES_DAY")
                .ok()
                .map(|s| s.parse().context("FETCH_INVOICES_DAY must be a number between 1-31"))
                .transpose()?,
            target_keywords: env::var("TARGET_KEYWORDS_TO_FETCH_AND_DOWNLOAD")
                .unwrap_or_else(|_| "invoice,invoices,fatura,faturas".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            start_date,
            end_date,
            debug_logs_enabled: env::var("DEBUG_LOGS_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true",
        };

        config.validate()?;
        Ok(config)
    }

    /// Parse date range using smart defaults
    /// Default: 1st of last month to today
    /// Example: If today is 2024-10-15, defaults to 2024-09-01 to 2024-10-15 (45 days)
    fn parse_date_range() -> Result<(NaiveDate, NaiveDate)> {
        // Always use default: 1st of last month to today
        let today = Local::now().date_naive();

        // Calculate 1st of last month
        let last_month = if today.month() == 1 {
            NaiveDate::from_ymd_opt(today.year() - 1, 12, 1)
                .context("Failed to calculate last month date")?
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() - 1, 1)
                .context("Failed to calculate last month date")?
        };

        info!("Using default date range: {} to {}", last_month, today);
        Ok((last_month, today))
    }

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        if let Some(day) = self.fetch_invoices_day {
            if day < 1 || day > 31 {
                anyhow::bail!("FETCH_INVOICES_DAY must be between 1 and 31");
            }
        }

        if self.gmail_client_id.is_empty() {
            anyhow::bail!("GOOGLE_GMAIL_CLIENT_ID cannot be empty");
        }

        if self.drive_client_id.is_empty() {
            anyhow::bail!("GOOGLE_DRIVE_CLIENT_ID cannot be empty");
        }

        if self.target_keywords.is_empty() {
            anyhow::bail!("TARGET_KEYWORDS_TO_FETCH_AND_DOWNLOAD must contain at least one keyword");
        }

        Ok(())
    }
}


