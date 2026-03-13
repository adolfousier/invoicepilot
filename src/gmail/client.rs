//! Gmail API HTTP client and response types.

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Base URL for the Gmail REST API v1.
pub const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1";

/// Authenticated HTTP client for the Gmail API.
#[derive(Debug, Clone)]
pub struct GmailClient {
    client: Client,
    access_token: String,
}

impl GmailClient {
    /// Create a new Gmail client with the given OAuth2 access token.
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }

    /// Returns a reference to the underlying HTTP client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the OAuth2 access token.
    pub fn access_token(&self) -> &str {
        &self.access_token
    }
}

/// Response from `users.messages.list` endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct MessageListResponse {
    /// List of message stubs (id + threadId).
    pub messages: Option<Vec<MessageInfo>>,
    /// Estimated total number of results.
    #[serde(rename = "resultSizeEstimate")]
    pub result_size_estimate: Option<u32>,
}

/// Minimal message reference returned by list endpoints.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageInfo {
    /// The immutable message ID.
    pub id: String,
    /// The thread this message belongs to.
    #[serde(rename = "threadId")]
    pub thread_id: String,
}

/// Full message resource from `users.messages.get`.
#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    /// The immutable message ID.
    pub id: String,
    /// Parsed MIME structure of the message.
    pub payload: Option<MessagePart>,
}

/// A single email header (name-value pair).
#[derive(Debug, Deserialize, Serialize)]
pub struct MessageHeader {
    /// Header name (e.g. "From", "Subject").
    pub name: String,
    /// Header value.
    pub value: String,
}

/// A MIME part within a message (may be nested).
#[derive(Debug, Deserialize, Serialize)]
pub struct MessagePart {
    /// Child MIME parts for multipart messages.
    pub parts: Option<Vec<MessagePart>>,
    /// Body content or attachment reference.
    pub body: Option<MessagePartBody>,
    /// MIME type (e.g. "application/pdf").
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    /// Filename for attachment parts.
    pub filename: Option<String>,
    /// Headers specific to this part.
    pub headers: Option<Vec<MessageHeader>>,
}

/// Body of a MIME part, either inline data or an attachment reference.
#[derive(Debug, Deserialize, Serialize)]
pub struct MessagePartBody {
    /// Attachment ID for downloading via the attachments endpoint.
    #[serde(rename = "attachmentId")]
    pub attachment_id: Option<String>,
    /// Base64url-encoded body data (for inline parts).
    pub data: Option<String>,
    /// Size in bytes.
    pub size: Option<u32>,
}

/// Raw attachment data returned by the attachments endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct Attachment {
    /// Base64url-encoded attachment content.
    pub data: String,
    /// Size in bytes.
    pub size: u32,
}
