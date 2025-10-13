use anyhow::{Context, Result};
use base64::prelude::*;
use std::path::PathBuf;
use super::client::{GmailClient, GMAIL_API_BASE, Message, Attachment, MessagePart};

#[derive(Debug, Clone)]
pub struct InvoiceAttachment {
    pub filename: String,
    pub data: Vec<u8>,
    #[allow(dead_code)]
    pub message_id: String,
}

/// Get message and extract all attachments
pub async fn get_message_attachments(
    client: &GmailClient,
    message_id: &str,
) -> Result<Vec<InvoiceAttachment>> {
    let url = format!("{}/users/me/messages/{}", GMAIL_API_BASE, message_id);

    let response = client.client()
        .get(&url)
        .bearer_auth(client.access_token())
        .send()
        .await
        .context("Failed to fetch message")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Gmail API error ({}): {}", status, error_text);
    }

    let message: Message = response.json().await
        .context("Failed to parse message")?;

    let mut attachments = Vec::new();

    if let Some(payload) = message.payload {
        find_attachments(&payload, &mut attachments);
    }

    if attachments.is_empty() {
        println!("   ⚠ No downloadable attachments found in message");
    }

    // Download attachment data
    let mut result = Vec::new();
    for (filename, attachment_id) in attachments {
        match download_attachment(client, message_id, &attachment_id).await {
            Ok(data) => {
                result.push(InvoiceAttachment {
                    filename: filename.clone(),
                    data,
                    message_id: message_id.to_string(),
                });
                println!("   ✓ Downloaded: {}", filename);
            }
            Err(e) => {
                eprintln!("   ✗ Failed to download {}: {}", filename, e);
            }
        }
    }

    Ok(result)
}

/// Recursively find all attachments in message parts
fn find_attachments(part: &MessagePart, attachments: &mut Vec<(String, String)>) {
    // Check if this part is an attachment
    // An attachment has a filename and an attachment_id
    if let Some(filename) = &part.filename {
        if !filename.is_empty() {
            if let Some(body) = &part.body {
                if let Some(attachment_id) = &body.attachment_id {
                    // This is an attachment - add it regardless of mime type
                    attachments.push((filename.clone(), attachment_id.clone()));
                }
            }
        }
    }

    // Recursively check child parts
    if let Some(parts) = &part.parts {
        for child_part in parts {
            find_attachments(child_part, attachments);
        }
    }
}

/// Download attachment data
async fn download_attachment(
    client: &GmailClient,
    message_id: &str,
    attachment_id: &str,
) -> Result<Vec<u8>> {
    let url = format!(
        "{}/users/me/messages/{}/attachments/{}",
        GMAIL_API_BASE, message_id, attachment_id
    );

    let response = client.client()
        .get(&url)
        .bearer_auth(client.access_token())
        .send()
        .await
        .context("Failed to download attachment")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Gmail API error ({}): {}", status, error_text);
    }

    let attachment: Attachment = response.json().await
        .context("Failed to parse attachment response")?;

    // Gmail API returns base64url-encoded data (RFC 4648 §5)
    // Try multiple base64 decoders in case of different formats
    let data = BASE64_URL_SAFE_NO_PAD.decode(attachment.data.as_bytes())
        .or_else(|_| BASE64_URL_SAFE.decode(attachment.data.as_bytes()))
        .or_else(|_| BASE64_STANDARD.decode(attachment.data.as_bytes()))
        .or_else(|_| {
            // Gmail sometimes returns data with URL-safe characters that need replacing
            let cleaned = attachment.data.replace('-', "+").replace('_', "/");
            BASE64_STANDARD.decode(cleaned.as_bytes())
        })
        .context(format!("Failed to decode attachment data (size: {})", attachment.data.len()))?;

    Ok(data)
}

/// Save attachment to temp directory
pub fn save_attachment_to_temp(attachment: &InvoiceAttachment) -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir().join("invoice-agent");
    std::fs::create_dir_all(&temp_dir)
        .context("Failed to create temp directory")?;

    let file_path = temp_dir.join(&attachment.filename);
    std::fs::write(&file_path, &attachment.data)
        .context("Failed to write attachment to temp file")?;

    Ok(file_path)
}
