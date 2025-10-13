use anyhow::{Context, Result};
use super::client::{DriveClient, DRIVE_API_BASE, FileListResponse, FileMetadata};

const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";

/// Find or create a folder by path (e.g., "billing/all-expenses/2025")
pub async fn find_or_create_folder(
    client: &DriveClient,
    folder_path: &str,
) -> Result<String> {
    println!("📁 Locating folder: {}...", folder_path);

    let parts: Vec<&str> = folder_path.split('/').filter(|s| !s.is_empty()).collect();

    if parts.is_empty() {
        anyhow::bail!("Folder path cannot be empty");
    }

    let mut parent_id = "root".to_string();

    for part in parts {
        parent_id = find_or_create_single_folder(client, part, &parent_id).await?;
    }

    println!("✓ Folder ready: {} (ID: {})", folder_path, parent_id);
    Ok(parent_id)
}

/// Find or create a single folder within a parent
async fn find_or_create_single_folder(
    client: &DriveClient,
    folder_name: &str,
    parent_id: &str,
) -> Result<String> {
    // Try to find existing folder
    if let Some(folder_id) = find_folder(client, folder_name, parent_id).await? {
        println!("   ✓ Found existing folder: {}", folder_name);
        return Ok(folder_id);
    }

    // Create new folder
    println!("   + Creating folder: {}", folder_name);
    create_folder(client, folder_name, parent_id).await
}

/// Search for a folder by name within a parent
async fn find_folder(
    client: &DriveClient,
    folder_name: &str,
    parent_id: &str,
) -> Result<Option<String>> {
    let query = format!(
        "name='{}' and '{}' in parents and mimeType='{}' and trashed=false",
        folder_name.replace("'", "\\'"),
        parent_id,
        FOLDER_MIME_TYPE
    );

    let url = format!("{}/files", DRIVE_API_BASE);

    let response = client.client()
        .get(&url)
        .bearer_auth(client.access_token())
        .query(&[("q", &query), ("fields", &"files(id, name)".to_string())])
        .send()
        .await
        .context("Failed to search for folder")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Drive API error ({}): {}", status, error_text);
    }

    let result: FileListResponse = response.json().await
        .context("Failed to parse folder search response")?;

    Ok(result.files.and_then(|files| files.first().map(|f| f.id.clone())))
}

/// Create a new folder
async fn create_folder(
    client: &DriveClient,
    folder_name: &str,
    parent_id: &str,
) -> Result<String> {
    let url = format!("{}/files", DRIVE_API_BASE);

    let metadata = FileMetadata {
        name: folder_name.to_string(),
        parents: Some(vec![parent_id.to_string()]),
        mime_type: Some(FOLDER_MIME_TYPE.to_string()),
    };

    let response = client.client()
        .post(&url)
        .bearer_auth(client.access_token())
        .json(&metadata)
        .send()
        .await
        .context("Failed to create folder")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Drive API error ({}): {}", status, error_text);
    }

    let created: serde_json::Value = response.json().await
        .context("Failed to parse folder creation response")?;

    let folder_id = created["id"].as_str()
        .context("Folder ID not found in response")?
        .to_string();

    Ok(folder_id)
}

#[cfg(test)]
mod tests {


    #[test]
    fn test_folder_path_parsing() {
        let path = "billing/all-expenses/2025";
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        assert_eq!(parts, vec!["billing", "all-expenses", "2025"]);
    }
}
