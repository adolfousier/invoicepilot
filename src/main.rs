mod auth;
mod cli;
mod config;
mod drive;
mod gmail;
mod scheduler;

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
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



/// Determine the billing month from the date range
/// If the range is primarily in one month, use that month
/// Otherwise, use the end date's month
fn determine_billing_month(start_date: NaiveDate, end_date: NaiveDate) -> String {
    // If range spans multiple months, use the month that contains most of the range
    let start_month = start_date.month();
    let end_month = end_date.month();

    if start_month == end_month {
        // Same month - use it
        format!("{}", chrono::Month::try_from(end_month as u8).unwrap().name())
    } else {
        // Different months - check if it's primarily last month billing
        let days_in_end_month = (end_date - NaiveDate::from_ymd_opt(end_date.year(), end_date.month(), 1).unwrap()).num_days() + 1;
        let total_days = (end_date - start_date).num_days() + 1;

        // If we're early in the end month (less than 15 days), it's likely previous month billing
        if days_in_end_month < 15 && total_days > 20 {
            format!("{}", chrono::Month::try_from(start_month as u8).unwrap().name())
        } else {
            format!("{}", chrono::Month::try_from(end_month as u8).unwrap().name())
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
        bank_groups.entry(attachment.bank_name.clone()).or_insert_with(Vec::new).push(attachment.clone());
    }

    let mut total_uploaded = 0;
    let mut total_failed = 0;
    let mut banks_processed = Vec::new();
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
        let bank_summary = drive::upload::upload_files(&drive_client, &file_paths, &bank_folder_id).await?;
        
        total_uploaded += bank_summary.uploaded;
        total_failed += bank_summary.failed;
        banks_processed.push((bank_display_name.to_string(), bank_summary.clone()));
        
        println!("   ✓ Bank summary: {} uploaded, {} failed", bank_summary.uploaded, bank_summary.failed);
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
    println!("Uploaded:       {}", total_uploaded);
    println!("Failed:         {}", total_failed);
    println!("Monthly folder: {}", monthly_folder_path);
    
    if !banks_processed.is_empty() {
        println!("\n🏦 Bank breakdown:");
        for (bank_name, summary) in &banks_processed {
            if bank_name == "General" {
                println!("  📄 General documents: {} uploaded, {} failed", summary.uploaded, summary.failed);
            } else {
                println!("  🏦 {}: {} uploaded, {} failed", bank_name, summary.uploaded, summary.failed);
            }
        }
    }
    
    // Show bank detection statistics
    let bank_count = banks_processed.iter().filter(|(name, _)| name != &"General").count();
    if bank_count > 0 {
        println!("\n✅ Detected {} bank statement(s) from different institutions", bank_count);
    } else {
        println!("\nℹ No bank statements detected - only general documents found");
    }

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
