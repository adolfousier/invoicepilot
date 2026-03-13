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
        bank_groups.entry(attachment.bank_name.clone()).or_default().push(attachment.clone());
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
        drive::upload::upload_files(&drive_client, &file_paths, &bank_folder_id, Some(tx)).await?;

        tx.send(format!("    ✓ {}: Files uploaded", bank_display_name))?;
    }

    // Cleanup temp files
    tx.send("Cleaning up temporary files...".to_string())?;

    for attachment in &all_attachments {
        if let Ok(path) = gmail::attachment::save_attachment_to_temp(&attachment.attachment)
            && let Err(e) = std::fs::remove_file(&path) {
                tx.send(format!("Failed to remove temp file {}: {}", path.display(), e))?;
            }
    }

    // Send completion summary
    tx.send(format!("__RESULTS__:processed={},month={},folder={}",
        all_attachments.len(), billing_month, monthly_folder_path))?;

    tx.send("Processing completed successfully!".to_string())?;

    Ok(())
}

/// Detect missing months on Google Drive and process them
pub async fn run_catchup_processing(
    tx: &mpsc::UnboundedSender<String>,
) -> Result<Vec<String>> {
    tx.send("Loading configuration...".to_string())?;
    let config = Config::from_env()?;

    tx.send("Authenticating with Google Drive...".to_string())?;
    let drive_token = crate::auth::drive_auth::get_drive_token(
        config.drive_client_id.clone(),
        config.drive_client_secret.clone(),
    )
    .await?;
    let drive_client = drive::client::DriveClient::new(drive_token);

    // Determine which year folders to check based on drive_folder_path
    // e.g., "Neura-AI-Billing/All-Expenses/2025" -> check 2025
    // We'll check the current year and previous year if we're in January
    let now = chrono::Utc::now();
    let current_year = now.year();
    let current_month = now.month();

    // Extract the base path (without year) and the year from config
    let folder_path = &config.drive_folder_path;

    // Try to detect year in the path, otherwise use current year
    let (base_path, years_to_check) = parse_folder_years(folder_path, current_year, current_month);

    let all_months = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];

    let mut missing_months: Vec<(i32, u32, String)> = Vec::new(); // (year, month_num, month_name)

    for year in &years_to_check {
        let year_folder_path = format!("{}/{}", base_path, year);
        tx.send(format!("🔍 Checking Drive folder: {}", year_folder_path))?;

        let existing_folders = drive::folder::list_subfolders(&drive_client, &year_folder_path).await?;
        tx.send(format!("  Found {} existing month folders: {:?}", existing_folders.len(), existing_folders))?;

        // Determine which months should exist for this year
        let max_month = if *year == current_year {
            // Only check up to previous month for current year
            if current_month == 1 { 0 } else { current_month - 1 }
        } else {
            12 // Check all months for past years
        };

        for month_num in 1..=max_month {
            let month_name = all_months[(month_num - 1) as usize];
            if !existing_folders.iter().any(|f| f.eq_ignore_ascii_case(month_name)) {
                missing_months.push((*year, month_num, month_name.to_string()));
            }
        }
    }

    if missing_months.is_empty() {
        tx.send("✅ No missing months detected! All months are up to date.".to_string())?;
        return Ok(Vec::new());
    }

    let missing_labels: Vec<String> = missing_months
        .iter()
        .map(|(y, _, m)| format!("{} {}", m, y))
        .collect();
    tx.send(format!("⚠️ Missing months detected: {}", missing_labels.join(", ")))?;

    // Process each missing month
    let mut processed_months = Vec::new();
    for (year, month_num, month_name) in &missing_months {
        tx.send(format!("📅 Processing {} {}...", month_name, year))?;

        let start_date = NaiveDate::from_ymd_opt(*year, *month_num, 1)
            .ok_or_else(|| anyhow::anyhow!("Invalid date for {} {}", month_name, year))?;

        let end_date = if *month_num == 12 {
            NaiveDate::from_ymd_opt(*year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(*year, *month_num + 1, 1)
        }
        .ok_or_else(|| anyhow::anyhow!("Invalid end date calculation"))?
        .pred_opt()
        .ok_or_else(|| anyhow::anyhow!("Invalid end date"))?;

        match run_manual_processing(start_date, end_date, tx).await {
            Ok(_) => {
                tx.send(format!("✅ {} {} processed successfully!", month_name, year))?;
                processed_months.push(format!("{} {}", month_name, year));
            }
            Err(e) => {
                tx.send(format!("❌ Failed to process {} {}: {}", month_name, year, e))?;
            }
        }
    }

    tx.send(format!(
        "__CATCHUP_RESULTS__:total_missing={},processed={}",
        missing_months.len(),
        processed_months.len()
    ))?;

    Ok(processed_months)
}

/// Parse the folder path to extract base path and years to check
pub(crate) fn parse_folder_years(folder_path: &str, current_year: i32, current_month: u32) -> (String, Vec<i32>) {
    let parts: Vec<&str> = folder_path.rsplitn(2, '/').collect();

    if parts.len() == 2 {
        // Try to parse the last segment as a year
        if let Ok(year) = parts[0].parse::<i32>() {
            let base = parts[1].to_string();
            let mut years = vec![year];
            // If configured year is current year and we're in Jan, also check previous year
            if year == current_year && current_month == 1 {
                years.insert(0, current_year - 1);
            }
            // If the configured year is older, include all years up to current
            if year < current_year {
                for y in (year + 1)..=current_year {
                    years.push(y);
                }
            }
            return (base, years);
        }
    }

    // Fallback: treat entire path as base, check current year
    let mut years = vec![current_year];
    if current_month == 1 {
        years.insert(0, current_year - 1);
    }
    (folder_path.to_string(), years)
}

/// Determine the billing month from the date range
pub(crate) fn determine_billing_month(start_date: NaiveDate, end_date: NaiveDate) -> String {
    let start_month = start_date.month();
    let end_month = end_date.month();

    if start_month == end_month {
        chrono::Month::try_from(end_month as u8).unwrap().name().to_string()
    } else {
        let days_in_end_month = (end_date - NaiveDate::from_ymd_opt(end_date.year(), end_date.month(), 1).unwrap()).num_days() + 1;
        let total_days = (end_date - start_date).num_days() + 1;

        if days_in_end_month < 15 && total_days > 20 {
            chrono::Month::try_from(start_month as u8).unwrap().name().to_string()
        } else {
            chrono::Month::try_from(end_month as u8).unwrap().name().to_string()
        }
    }
}