use anyhow::{Context, Result};
use reqwest::multipart::{Form, Part};
use std::path::Path;
use tokio::sync::mpsc;
use super::client::{DriveClient, DRIVE_UPLOAD_BASE, FileMetadata, UploadedFile, FileListResponse, DRIVE_API_BASE};

/// Upload a file to Google Drive
pub async fn upload_file(
    client: &DriveClient,
    file_path: &Path,
    folder_id: &str,
    skip_duplicates: bool,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<UploadedFile> {
    let filename = file_path.file_name()
        .context("Invalid file path")?
        .to_string_lossy()
        .to_string();

    // Check for duplicates if requested
    if skip_duplicates {
        if let Some(existing_file) = find_file_in_folder(client, &filename, folder_id).await? {
            let _ = tx.send(format!("   ⚠ Skipping duplicate: {} (already exists)", filename));
            return Ok(existing_file);
        }
    }

    let _ = tx.send(format!("   ↑ Uploading: {}...", filename));

    let file_data = std::fs::read(file_path)
        .context("Failed to read file")?;

    let metadata = FileMetadata {
        name: filename.clone(),
        parents: Some(vec![folder_id.to_string()]),
        mime_type: Some("application/pdf".to_string()),
    };

    let metadata_json = serde_json::to_string(&metadata)
        .context("Failed to serialize metadata")?;

    // Create multipart form
    let metadata_part = Part::text(metadata_json)
        .mime_str("application/json")?;

    let file_part = Part::bytes(file_data)
        .file_name(filename.clone())
        .mime_str("application/pdf")?;

    let form = Form::new()
        .part("metadata", metadata_part)
        .part("file", file_part);

    let url = format!("{}/files?uploadType=multipart", DRIVE_UPLOAD_BASE);

    let response = client.client()
        .post(&url)
        .bearer_auth(client.access_token())
        .multipart(form)
        .send()
        .await
        .context("Failed to upload file")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Drive API error ({}): {}", status, error_text);
    }

    let uploaded: UploadedFile = response.json().await
        .context("Failed to parse upload response")?;

    let _ = tx.send(format!("   ✓ Uploaded: {} (ID: {})", filename, uploaded.id));
    Ok(uploaded)
}

/// Find a file by name in a specific folder
async fn find_file_in_folder(
    client: &DriveClient,
    filename: &str,
    folder_id: &str,
) -> Result<Option<UploadedFile>> {
    let query = format!(
        "name='{}' and '{}' in parents and trashed=false",
        filename.replace("'", "\\'"),
        folder_id
    );

    let url = format!("{}/files", DRIVE_API_BASE);

    let response = client.client()
        .get(&url)
        .bearer_auth(client.access_token())
        .query(&[("q", &query), ("fields", &"files(id, name, webViewLink)".to_string())])
        .send()
        .await
        .context("Failed to search for file")?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let result: FileListResponse = response.json().await
        .context("Failed to parse file search response")?;

    if let Some(files) = result.files {
        if let Some(file) = files.first() {
            return Ok(Some(UploadedFile {
                id: file.id.clone(),
                name: file.name.clone(),
                web_view_link: None,
            }));
        }
    }

    Ok(None)
}

/// Upload multiple files and return summary
pub async fn upload_files(
    client: &DriveClient,
    file_paths: &[std::path::PathBuf],
    folder_id: &str,
    tx: &mpsc::UnboundedSender<String>,
) -> Result<UploadSummary> {
    for file_path in file_paths {
        match upload_file(client, file_path, folder_id, true, tx).await {
            Ok(_) => {
                // File uploaded successfully
            },
            Err(e) => {
                let _ = tx.send(format!("   ✗ Failed to upload {}: {}", file_path.display(), e));
            }
        }
    }

    Ok(UploadSummary {})
}

#[derive(Debug, Clone)]
pub struct UploadSummary {}
