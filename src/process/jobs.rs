use crate::auth;
use crate::config::env::Config;
use crate::drive;
use crate::gmail;
use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub async fn run_manual_processing(
    start_date: NaiveDate,
    end_date: NaiveDate,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<()> {
    tx.send("Loading configuration...".to_string())?;
    let config = Config::from_env()?;

    tx.send("Authenticating with Gmail...".to_string())?;

    let gmail_token = auth::gmail_auth::get_gmail_token(
        config.gmail_client_id.clone(),
        config.gmail_client_secret.clone(),
    )
    .await?;
    let gmail_client = gmail::client::GmailClient::new(gmail_token);

    tx.send("Authenticating with Google Drive...".to_string())?;

    let drive_token = auth::drive_auth::get_drive_token(
        config.drive_client_id.clone(),
        config.drive_client_secret.clone(),
    )
    .await?;
    let drive_client = drive::client::DriveClient::new(drive_token);

    tx.send(format!("🔍 Searching Gmail for invoices and bank statements from {} to {}...", start_date, end_date))?;

    let message_ids = gmail::search::search_invoices(&gmail_client, start_date, end_date, &config.target_keywords).await?;

    if message_ids.is_empty() {
        tx.send("No invoices found in the specified date range".to_string())?;
        return Ok(());
    }

    tx.send(format!("✓ Found {} unique message(s) with potential invoices", message_ids.len()))?;
    tx.send("⬇️ Downloading attachments...".to_string())?;

    let mut all_attachments = Vec::new();
    for (idx, message_id) in message_ids.iter().enumerate() {
        tx.send(format!("  Processing message {}/{}", idx + 1, message_ids.len()))?;

        match gmail::attachment::get_message_attachments(&gmail_client, message_id).await {
            Ok(attachments) => {
                if attachments.is_empty() {
                    tx.send("      ⚠ No attachments in this message".to_string())?;
                } else {
                    for attachment in &attachments {
                        if let Some(ref bank) = attachment.bank_name {
                            tx.send(format!("      ✓ {}: {} (🏦 {})", attachment.attachment.filename.len(), attachment.attachment.filename, bank))?;
                        } else {
                            tx.send(format!("      ✓ {}: {} (📄 General)", attachment.attachment.filename.len(), attachment.attachment.filename))?;
                        }
                    }
                }
                all_attachments.extend(attachments);
            }
            Err(e) => {
                tx.send(format!("      ✗ Failed to process message: {}", e))?;
            }
        }
    }

    if all_attachments.is_empty() {
        tx.send("No attachments found in messages".to_string())?;
        return Ok(());
    }

    tx.send(format!("Downloaded {} attachment(s)", all_attachments.len()))?;
    tx.send("Preparing upload...".to_string())?;

    // Determine billing month
    let billing_month = determine_billing_month(start_date, end_date);
    tx.send(format!("Billing month detected: {}", billing_month))?;

    let monthly_folder_path = format!("{}/{}", config.drive_folder_path, billing_month);
    let _monthly_folder_id = drive::folder::find_or_create_folder(&drive_client, &monthly_folder_path).await?;

    // Group attachments by bank name
    let mut bank_groups: HashMap<Option<String>, Vec<gmail::attachment::InvoiceAttachmentWithBank>> = HashMap::new();
    for attachment in &all_attachments {
        bank_groups.entry(attachment.bank_name.clone()).or_insert_with(Vec::new).push(attachment.clone());
    }

    tx.send("⬆️ Uploading to Google Drive...".to_string())?;

    // Upload files to bank-specific folders
    for (bank_name, attachments) in bank_groups {
        let bank_display_name = bank_name.as_deref().unwrap_or("General");
        tx.send(format!("  🏦 Processing bank: {}", bank_display_name))?;

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
                }
                Err(e) => {
                    tx.send(format!("    ✗ Failed to save {}: {}", attachment.attachment.filename, e))?;
                }
            }
        }

        // Upload files to bank-specific folder
        drive::upload::upload_files(&drive_client, &file_paths, &bank_folder_id, Some(&tx)).await?;

        tx.send(format!("    ✓ {}: Files uploaded", bank_display_name))?;
    }

    // Cleanup temp files
    tx.send("Cleaning up temporary files...".to_string())?;

    for attachment in &all_attachments {
        if let Ok(path) = gmail::attachment::save_attachment_to_temp(&attachment.attachment) {
            if let Err(e) = std::fs::remove_file(&path) {
                tx.send(format!("Failed to remove temp file {}: {}", path.display(), e))?;
            }
        }
    }

    // Send completion summary
    tx.send(format!("__RESULTS__:processed={},month={},folder={}",
        all_attachments.len(), billing_month, monthly_folder_path))?;

    tx.send("Processing completed successfully!".to_string())?;

    Ok(())
}

/// Determine the billing month from the date range
fn determine_billing_month(start_date: NaiveDate, end_date: NaiveDate) -> String {
    let start_month = start_date.month();
    let end_month = end_date.month();

    if start_month == end_month {
        format!("{}", chrono::Month::try_from(end_month as u8).unwrap().name())
    } else {
        let days_in_end_month = (end_date - NaiveDate::from_ymd_opt(end_date.year(), end_date.month(), 1).unwrap()).num_days() + 1;
        let total_days = (end_date - start_date).num_days() + 1;

        if days_in_end_month < 15 && total_days > 20 {
            format!("{}", chrono::Month::try_from(start_month as u8).unwrap().name())
        } else {
            format!("{}", chrono::Month::try_from(end_month as u8).unwrap().name())
        }
    }
}