mod app;
mod auth;
mod cli;
mod config;
mod db;
mod drive;
mod gmail;
mod process;
mod scheduler;
mod interfaces;

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use clap::{Parser, Subcommand};
use config::env::Config;
use std::fs;

#[derive(Parser, Debug)]
#[command(name = "invoice-pilot")]
#[command(about = "Automated invoice fetcher from Gmail to Google Drive", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run in interactive TUI mode (default)
    Tui,
    /// Run manually (legacy CLI mode)
    Manual {
        /// Custom date range in format YYYY-MM-DD:YYYY-MM-DD
        #[arg(short, long)]
        date_range: Option<String>,
    },
    /// Run in scheduled mode (legacy CLI mode)
    Scheduled,
    /// Manage authentication tokens (legacy CLI mode)
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
}

#[derive(Subcommand, Debug)]
enum AuthAction {
    /// Re-authenticate Gmail account
    Gmail,
    /// Re-authenticate Google Drive account
    Drive,
    /// Clear all tokens (force re-authentication for both)
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create logs directory if it doesn't exist
    fs::create_dir_all("src/data/logs").ok();

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Tui => {
            // For TUI mode, only log to file if debug logging is enabled
            // Never log to console to avoid interfering with TUI
            if let Ok(config) = Config::from_env()
                && config.debug_logs_enabled {
                    let _ = init_file_logging_only();
                }
            // Run the interactive TUI
            if let Err(e) = interfaces::tui::run_tui().await {
                eprintln!("TUI error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Manual { date_range } => {
            run_manual(date_range).await?;
        }
        Commands::Scheduled => {
            run_scheduled_legacy().await?;
        }
        Commands::Auth { action } => {
            handle_auth_command(action).await?;
        }
    }

    Ok(())
}

/// Initialize logging with only file output (no console) for TUI mode
fn init_file_logging_only() -> Result<()> {
    // Create a simple file-only logger configuration
    let file = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new("{d} - {l} - {t} - {m}{n}")))
        .build("src/data/logs/server.log")?;

    let config = log4rs::config::Config::builder()
        .appender(log4rs::config::Appender::builder().build("file", Box::new(file)))
        .build(log4rs::config::Root::builder()
            .appender("file")
            .build(log::LevelFilter::Info))?;

    log4rs::init_config(config)?;

    Ok(())
}

async fn run_manual(date_range: Option<String>) -> Result<()> {
    println!("🚀 Invoice Agent - Manual Mode\n");

    // Load configuration
    let config = Config::from_env()?;

    // Determine date range - prioritize CLI arg, then config (FILTER_BY_DATE or smart default)
    let (start_date, end_date) = if let Some(range_str) = date_range {
        println!("📅 Using CLI-provided date range");
        scheduler::runner::parse_date_range(&range_str)?
    } else {
        // Use dates from config (already parsed from FILTER_BY_DATE or defaults)
        (config.start_date, config.end_date)
    };

    println!("📅 Date range: {} to {}\n", start_date, end_date);

    // Execute the invoice fetching pipeline
    fetch_and_upload_invoices(config, start_date, end_date).await?;

    println!("\n✅ Manual run completed successfully!");
    Ok(())
}

async fn run_scheduled_legacy() -> Result<()> {
    println!("⏰ Invoice Agent - Scheduled Mode\n");

    // Load configuration
    let config = Config::from_env()?;

    // Validate that FETCH_INVOICES_DAY is set for scheduled mode
    let fetch_invoices_day = config.fetch_invoices_day
        .ok_or_else(|| anyhow::anyhow!("FETCH_INVOICES_DAY must be set in .env for scheduled mode"))?;

    // Check if we should run today
    if !scheduler::runner::should_run_today(fetch_invoices_day) {
        println!("ℹ Not scheduled to run today (runs on day {})", fetch_invoices_day);
        println!("Current day: {}", chrono::Utc::now().day());
        return Ok(());
    }

    // Run directly (legacy mode always runs directly)
    println!("✓ Today is day {} - running invoice fetch\n", fetch_invoices_day);

    // Use previous month range
    let (start_date, end_date) = scheduler::runner::get_previous_month_range();
    println!("📅 Date range: {} to {}\n", start_date, end_date);

    // Execute the invoice fetching pipeline
    fetch_and_upload_invoices(config, start_date, end_date).await?;

    println!("\n✅ Scheduled run completed successfully!");
    Ok(())
}

/// Determine the billing month from the date range
/// If the range is primarily in one month, use that month
/// Otherwise, use the end date's month
fn determine_billing_month(start_date: NaiveDate, end_date: NaiveDate) -> String {
    // If range spans multiple months, use the month that contains most of the range
    let start_month = start_date.month();
    let end_month = end_date.month();

    if start_month == end_month {
        // Same month - use it
        chrono::Month::try_from(end_month as u8).unwrap().name().to_string()
    } else {
        // Different months - check if it's primarily last month billing
        let days_in_end_month = (end_date - NaiveDate::from_ymd_opt(end_date.year(), end_date.month(), 1).unwrap()).num_days() + 1;
        let total_days = (end_date - start_date).num_days() + 1;

        // If we're early in the end month (less than 15 days), it's likely previous month billing
        if days_in_end_month < 15 && total_days > 20 {
            chrono::Month::try_from(start_month as u8).unwrap().name().to_string()
        } else {
            chrono::Month::try_from(end_month as u8).unwrap().name().to_string()
        }
    }
}

async fn fetch_and_upload_invoices(
    config: Config,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<()> {
    // 1. Authenticate with Gmail
    println!("═══ Gmail Authentication ═══");
    let gmail_token = auth::gmail_auth::get_gmail_token(
        config.gmail_client_id.clone(),
        config.gmail_client_secret.clone(),
    )
    .await?;
    let gmail_client = gmail::client::GmailClient::new(gmail_token);

    // 2. Authenticate with Drive
    println!("\n═══ Google Drive Authentication ═══");
    let drive_token = auth::drive_auth::get_drive_token(
        config.drive_client_id.clone(),
        config.drive_client_secret.clone(),
    )
    .await?;
    let drive_client = drive::client::DriveClient::new(drive_token);

    // 3. Search Gmail for invoices
    println!("\n═══ Searching Gmail ═══");
    let message_ids = gmail::search::search_invoices(&gmail_client, start_date, end_date, &config.target_keywords).await?;

    if message_ids.is_empty() {
        println!("\nℹ No invoices found in the specified date range");
        return Ok(());
    }

    // 4. Download attachments
    println!("\n═══ Downloading Attachments ═══");
    let mut all_attachments = Vec::new();

    for (idx, message_id) in message_ids.iter().enumerate() {
        println!("Processing message {}/{}: {}", idx + 1, message_ids.len(), message_id);

        match gmail::attachment::get_message_attachments(&gmail_client, message_id).await {
            Ok(attachments) => {
                all_attachments.extend(attachments);
            }
            Err(e) => {
                eprintln!("   ✗ Failed to process message {}: {}", message_id, e);
            }
        }
    }

    if all_attachments.is_empty() {
        println!("\nℹ No attachments found in messages");
        return Ok(());
    }

    println!("\n✓ Downloaded {} attachment(s)", all_attachments.len());

    // 5. Determine billing month and create monthly folder
    let billing_month = determine_billing_month(start_date, end_date);
    println!("📅 Billing month detected: {}", billing_month);

    let monthly_folder_path = format!("{}/{}", config.drive_folder_path, billing_month);
    let _monthly_folder_id = drive::folder::find_or_create_folder(&drive_client, &monthly_folder_path).await?;

    // 6. Group attachments by bank name and prepare for upload
    println!("\n═══ Preparing Upload ═══");
    
    // Group attachments by bank name
    let mut bank_groups: std::collections::HashMap<Option<String>, Vec<gmail::attachment::InvoiceAttachmentWithBank>> = std::collections::HashMap::new();
    for attachment in &all_attachments {
        bank_groups.entry(attachment.bank_name.clone()).or_default().push(attachment.clone());
    }

    let mut all_file_paths = Vec::new();

    // 7. Upload files to bank-specific folders
    println!("\n═══ Uploading to Google Drive ═══");
    
    for (bank_name, attachments) in bank_groups {
        let bank_display_name = bank_name.as_deref().unwrap_or("General");
        println!("\n🏦 Processing bank: {}", bank_display_name);
        
        // Create bank-specific folder
        let bank_folder_path = if let Some(ref bank) = bank_name {
            format!("{}/{}", monthly_folder_path, bank)
        } else {
            monthly_folder_path.clone()
        };
        
        let bank_folder_id = drive::folder::find_or_create_folder(&drive_client, &bank_folder_path).await?;
        
        // Save attachments to temp directory for this bank
        let mut file_paths = Vec::new();
        for attachment in &attachments {
            match gmail::attachment::save_attachment_to_temp(&attachment.attachment) {
                Ok(path) => {
                    file_paths.push(path.clone());
                    all_file_paths.push(path);
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to save {}: {}", attachment.attachment.filename, e);
                }
            }
        }
        
        // Upload files to bank-specific folder
        drive::upload::upload_files(&drive_client, &file_paths, &bank_folder_id, None).await?;

        println!("   ✓ Bank: {} - Files uploaded", bank_display_name);
    }

    // 8. Cleanup temp files
    println!("\n═══ Cleanup ═══");
    for file_path in &all_file_paths {
        if let Err(e) = std::fs::remove_file(file_path) {
            eprintln!("   ⚠ Failed to remove temp file {}: {}", file_path.display(), e);
        }
    }
    println!("✓ Temp files cleaned up");

    // Print summary
    println!("\n═══ Summary ═══");
    println!("Total files:    {}", all_file_paths.len());
    println!("Monthly folder: {}", monthly_folder_path);

    Ok(())
}



async fn handle_auth_command(action: AuthAction) -> Result<()> {
    match action {
        AuthAction::Gmail => {
            println!("🔄 Re-authenticating Gmail...\n");
            auth::gmail_auth::clear_gmail_token()?;

            let config = Config::from_env()?;
            auth::gmail_auth::get_gmail_token(
                config.gmail_client_id,
                config.gmail_client_secret,
            )
            .await?;

            println!("\n✅ Gmail re-authenticated successfully!");
        }
        AuthAction::Drive => {
            println!("🔄 Re-authenticating Google Drive...\n");
            auth::drive_auth::clear_drive_token()?;

            let config = Config::from_env()?;
            auth::drive_auth::get_drive_token(
                config.drive_client_id,
                config.drive_client_secret,
            )
            .await?;

            println!("\n✅ Google Drive re-authenticated successfully!");
        }
        AuthAction::Reset => {
            println!("🔄 Resetting all authentication tokens...\n");
            auth::gmail_auth::clear_gmail_token()?;
            auth::drive_auth::clear_drive_token()?;
            println!("\n✅ All tokens cleared! Run manual or scheduled mode to re-authenticate.");
        }
    }

    Ok(())
}
