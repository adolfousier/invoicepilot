use reqwest::Client;
use serde::{Deserialize, Serialize};

pub const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1";

#[derive(Debug, Clone)]
pub struct GmailClient {
    client: Client,
    access_token: String,
}

impl GmailClient {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageListResponse {
    pub messages: Option<Vec<MessageInfo>>,
    #[serde(rename = "resultSizeEstimate")]
    pub result_size_estimate: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageInfo {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub id: String,
    pub payload: Option<MessagePart>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessagePart {
    pub parts: Option<Vec<MessagePart>>,
    pub body: Option<MessagePartBody>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub headers: Option<Vec<MessageHeader>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessagePartBody {
    #[serde(rename = "attachmentId")]
    pub attachment_id: Option<String>,
    pub data: Option<String>,
    pub size: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Attachment {
    pub data: String,
    pub size: u32,
}
