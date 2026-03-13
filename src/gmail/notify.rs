use anyhow::{Context, Result};
use base64::prelude::*;
use super::client::{GmailClient, GMAIL_API_BASE};

/// Send a notification email via Gmail API using the authenticated user's account
pub async fn send_notification(
    client: &GmailClient,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    let raw_email = format!(
        "To: {}\r\nSubject: {}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}",
        to, subject, body
    );

    let encoded = BASE64_URL_SAFE_NO_PAD.encode(raw_email.as_bytes());

    let url = format!("{}/users/me/messages/send", GMAIL_API_BASE);

    let payload = serde_json::json!({
        "raw": encoded
    });

    let response = client.client()
        .post(&url)
        .bearer_auth(client.access_token())
        .json(&payload)
        .send()
        .await
        .context("Failed to send notification email")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Gmail send error ({}): {}", status, error_text);
    }

    Ok(())
}

/// Build a processing completion summary email body
pub fn build_completion_body(
    processed: usize,
    uploaded: usize,
    failed: usize,
    folder: &str,
    months: &[String],
) -> String {
    let mut body = String::from("Invoice Pilot - Processing Complete\n");
    body.push_str("════════════════════════════════════\n\n");

    if !months.is_empty() {
        body.push_str(&format!("Months processed: {}\n", months.join(", ")));
    }

    body.push_str(&format!("Files processed: {}\n", processed));
    body.push_str(&format!("Files uploaded:   {}\n", uploaded));
    body.push_str(&format!("Failed:           {}\n", failed));
    body.push_str(&format!("\nDrive folder: {}\n", folder));
    body.push_str(&format!("\nTimestamp: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    body
}
