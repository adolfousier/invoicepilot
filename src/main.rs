mod auth;
mod cli;
mod config;
mod drive;
mod gmail;
mod scheduler;

use anyhow::Result;
use chrono::Datelike;
use clap::Parser;
use cli::args::{AuthAction, Cli, Commands};
use config::env::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Manual { date_range } => {
            run_manual(date_range).await?;
        }
        Commands::Scheduled => {
            run_scheduled().await?;
        }
        Commands::Auth { action } => {
            handle_auth_command(action).await?;
        }
    }

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

async fn run_scheduled() -> Result<()> {
    println!("⏰ Invoice Agent - Scheduled Mode\n");

    // Load configuration
    let config = Config::from_env()?;

    // Check if we should run today
    if !scheduler::runner::should_run_today(config.fetch_invoices_day) {
        println!("ℹ Not scheduled to run today (runs on day {})", config.fetch_invoices_day);
        println!("Current day: {}", chrono::Utc::now().day());
        return Ok(());
    }

    println!("✓ Today is day {} - running invoice fetch\n", config.fetch_invoices_day);

    // Use previous month range
    let (start_date, end_date) = scheduler::runner::get_previous_month_range();
    println!("📅 Date range: {} to {}\n", start_date, end_date);

    // Execute the invoice fetching pipeline
    fetch_and_upload_invoices(config, start_date, end_date).await?;

    println!("\n✅ Scheduled run completed successfully!");
    Ok(())
}

async fn fetch_and_upload_invoices(
    config: Config,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
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

    // 5. Save attachments to temp directory
    println!("\n═══ Preparing Upload ═══");
    let mut file_paths = Vec::new();

    for attachment in &all_attachments {
        match gmail::attachment::save_attachment_to_temp(attachment) {
            Ok(path) => {
                file_paths.push(path);
            }
            Err(e) => {
                eprintln!("   ✗ Failed to save {}: {}", attachment.filename, e);
            }
        }
    }

    // 6. Find or create Drive folder
    let folder_id = drive::folder::find_or_create_folder(&drive_client, &config.drive_folder_path).await?;

    // 7. Upload files to Drive
    println!("\n═══ Uploading to Google Drive ═══");
    let summary = drive::upload::upload_files(&drive_client, &file_paths, &folder_id).await?;

    // 8. Cleanup temp files
    println!("\n═══ Cleanup ═══");
    for file_path in &file_paths {
        if let Err(e) = std::fs::remove_file(file_path) {
            eprintln!("   ⚠ Failed to remove temp file {}: {}", file_path.display(), e);
        }
    }
    println!("✓ Temp files cleaned up");

    // Print summary
    println!("\n═══ Summary ═══");
    println!("Total files:    {}", summary.total);
    println!("Uploaded:       {}", summary.uploaded);
    println!("Failed:         {}", summary.failed);
    println!("Folder:         {}", config.drive_folder_path);

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
