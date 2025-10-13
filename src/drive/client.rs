use reqwest::Client;
use serde::{Deserialize, Serialize};

pub const DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
pub const DRIVE_UPLOAD_BASE: &str = "https://www.googleapis.com/upload/drive/v3";

#[derive(Debug, Clone)]
pub struct DriveClient {
    client: Client,
    access_token: String,
}

impl DriveClient {
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
pub struct FileListResponse {
    pub files: Option<Vec<FileInfo>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileMetadata {
    pub name: String,
    pub parents: Option<Vec<String>>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadedFile {
    pub id: String,
    #[allow(dead_code)]
    pub name: String,
    #[serde(rename = "webViewLink")]
    #[allow(dead_code)]
    pub web_view_link: Option<String>,
}
