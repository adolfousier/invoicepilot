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

#[derive(Debug, Clone)]
pub struct InvoiceAttachmentWithBank {
    pub attachment: InvoiceAttachment,
    pub bank_name: Option<String>,
}

/// Get message and extract all attachments
pub async fn get_message_attachments(
    client: &GmailClient,
    message_id: &str,
) -> Result<Vec<InvoiceAttachmentWithBank>> {
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

    // Extract sender name and detect bank from headers
    let sender_name = extract_sender_name(&message);
    let sender_prefix = sanitize_sender_name(&sender_name);
    let bank_name = detect_bank_name(&message);

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
                // Prepend sender name to filename
                let new_filename = if !sender_prefix.is_empty() {
                    format!("{}-{}", sender_prefix, filename)
                } else {
                    filename.clone()
                };

                let attachment_with_bank = InvoiceAttachmentWithBank {
                    attachment: InvoiceAttachment {
                        filename: new_filename.clone(),
                        data,
                        message_id: message_id.to_string(),
                    },
                    bank_name: bank_name.clone(),
                };

                // Log bank detection
                if let Some(ref bank) = bank_name {
                    println!("   ✓ Downloaded: {} (🏦 Bank: {})", new_filename, bank);
                } else {
                    println!("   ✓ Downloaded: {} (📄 General document)", new_filename);
                }

                result.push(attachment_with_bank);
            }
            Err(e) => {
                eprintln!("   ✗ Failed to download {}: {}", filename, e);
            }
        }
    }

    Ok(result)
}

/// Extract sender name from message headers
fn extract_sender_name(message: &Message) -> String {
    if let Some(payload) = &message.payload {
        if let Some(headers) = &payload.headers {
            for header in headers {
                if header.name.to_lowercase() == "from" {
                    // Extract name from "Name <email@example.com>" format
                    let from = &header.value;

                    // Try to extract the name part before the email
                    if let Some(name_end) = from.find('<') {
                        let name = from[..name_end].trim();
                        if !name.is_empty() {
                            // Remove quotes if present
                            return name.trim_matches('"').to_string();
                        }
                    }

                    // If no angle bracket, try to extract from email
                    if let Some(at_pos) = from.find('@') {
                        return from[..at_pos].to_string();
                    }

                    return from.clone();
                }
            }
        }
    }
    String::new()
}

/// Sanitize sender name for use in filename
/// "LangFuse GmbH" -> "langfuse-gmbh"
fn sanitize_sender_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                // Remove special characters
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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

/// Detect bank name from message headers and content
fn detect_bank_name(message: &Message) -> Option<String> {
    let search_text = extract_search_text(message);
    detect_bank_from_text(&search_text).map(|name| {
        // Convert to title case for consistent folder naming
        name.split_whitespace()
            .map(|word| {
                if word.len() > 0 {
                    format!("{}{}", word.chars().next().unwrap().to_uppercase(), word[1..].to_lowercase())
                } else {
                    word.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// Extract searchable text from message (headers + body)
fn extract_search_text(message: &Message) -> String {
    let mut text = String::new();
    
    // Extract from headers
    if let Some(payload) = &message.payload {
        if let Some(headers) = &payload.headers {
            for header in headers {
                if header.name.to_lowercase() == "from" || header.name.to_lowercase() == "subject" {
                    text.push_str(&header.value);
                    text.push(' ');
                }
            }
        }
        
        // Extract from body if available
        if let Some(body) = &payload.body {
            if let Some(data) = &body.data {
                // Try to decode and extract text from body
                // This is a simplified version - in production you'd want proper MIME parsing
                text.push_str(data);
            }
        }
    }
    
    text.to_lowercase()
}

/// Detect bank name from text using predefined patterns
fn detect_bank_from_text(text: &str) -> Option<String> {
    // Common European digital and physical banks (including those without 'bank' in name)
    let bank_patterns = vec![
        // Digital banks
        "wise", "revolut", "nubank", "bunq", "monzo", "starling", "chime", "venmo",
        "paypal", "wise", "transferwise", "wise.com", "revolut.com", "nubank.com.br",
        
        // Traditional banks with 'bank' in name
        "santander", "bbva", "caixabank", "ing", "deutsche bank", "commerzbank",
        "hsbc", "barclays", "lloyds", "rbs", "natwest", "barclays", "standard chartered",
        "bnp paribas", "societe generale", "credit agricole", "dexia", "fortis",
        "kbc", "rabobank", "abn amro", "ing", "asn", "triodos", "moneco",
        
        // Spanish banks
        "banco santander", "bbva", "caixa bank", "la caixa", "bankinter", "sabadell",
        "popular", "galicia", "santanderrio", "macro", "hipotecario", "provincia",
        
        // Portuguese banks
        "bcp", "bpi", "caixa geral de depósitos", "millennium bcp", "banco espírito santo",
        
        // Italian banks
        "intesa sanpaolo", "unicredit", "banco popolare", "monte dei paschi", "mediolanum",
        
        // French banks
        "societe generale", "bnp paribas", "credit agricole", "lcl", "bpce", "caisse d'epargne",
        
        // German banks
        "deutsche bank", "commerzbank", "hypovereinsbank", "sparkasse", "volksbank",
        
        // Dutch banks
        "ing", "rabobank", "abn amro", "asn bank", "triodos bank", "moneco bank",
        
        // Polish banks
        "pkobp", "ing", "millennium", "bnp paribas", "santander", "bank millennium",
        
        // Czech banks
        "csob", "kb", "unicredit", "raiffeisen", "moneta", "fio",
        
        // Austrian banks
        "erste bank", "raiffeisen", "bank austria", "volksbank", "sparkasse",
        
        // Swiss banks
        "ubs", "credit suisse", "zkb", "ubs", "postfinance", "raiffeisen",
        
        // Nordic banks
        "nordea", "dnb", "handelsbanken", "seb", "swedbank", "sampo",
        
        // Other European digital services
        "wise", "transferwise", "revolut", "n26", "bunq", "monzo", "starling",
        "tidal", "october", "bunq", "mollie", "adyen", "stripe", "paypal",
        
        // Brokerages and trading platforms
        "interactive brokers", "ibkr", "charles schwab", "etrade", "td ameritrade", " fidelity",
        "robinhood", "webull", "coinbase", "binance", "kraken", "coinbase pro", "binance us",
        
        // Banks with 'banco' in name (Spanish/Portuguese)
        "banco", "banco santander", "banco do brasil", "banco itaú", "banco bradesco",
        
        // Generic bank indicators
        "bank", "banco", "financial", "fintech", "fiscal", "tributary",
    ];
    
    for pattern in bank_patterns {
        if text.contains(&pattern.to_lowercase()) {
            return Some(pattern.to_string());
        }
    }
    
    None
}
