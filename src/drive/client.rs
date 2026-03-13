//! Google Drive API HTTP client and response types.

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Base URL for the Google Drive REST API v3.
pub const DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";

/// Base URL for Google Drive multipart/resumable uploads.
pub const DRIVE_UPLOAD_BASE: &str = "https://www.googleapis.com/upload/drive/v3";

/// Authenticated HTTP client for the Google Drive API.
#[derive(Debug, Clone)]
pub struct DriveClient {
    client: Client,
    access_token: String,
}

impl DriveClient {
    /// Create a new Drive client with the given OAuth2 access token.
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

/// Response from the `files.list` endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct FileListResponse {
    /// Matched files, if any.
    pub files: Option<Vec<FileInfo>>,
}

/// Metadata for a file or folder on Google Drive.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileInfo {
    /// The unique file ID.
    pub id: String,
    /// Display name of the file.
    pub name: String,
    /// MIME type (e.g. `application/vnd.google-apps.folder`).
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// Metadata payload for creating or updating a file on Drive.
#[derive(Debug, Serialize)]
pub struct FileMetadata {
    /// Desired filename.
    pub name: String,
    /// Parent folder IDs.
    pub parents: Option<Vec<String>>,
    /// MIME type of the file content.
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// Response after a successful file upload.
#[derive(Debug, Deserialize)]
pub struct UploadedFile {
    /// The newly created file's ID.
    pub id: String,
    /// Filename as stored on Drive.
    #[allow(dead_code)]
    pub name: String,
    /// Web link to view the file in Drive.
    #[serde(rename = "webViewLink")]
    #[allow(dead_code)]
    pub web_view_link: Option<String>,
}
