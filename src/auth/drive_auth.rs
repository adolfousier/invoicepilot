use anyhow::Result;
use log::info;
use oauth2::TokenResponse;
use super::oauth::{
    TokenCache, create_oauth_client, get_config_dir, load_token, save_token,
    perform_oauth_flow, refresh_token,
};
use std::fs;

const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const DRIVE_TOKEN_FILE: &str = "drive_token.json";

/// Get or refresh Drive access token
pub async fn get_drive_token(client_id: String, client_secret: String) -> Result<String> {
    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(DRIVE_TOKEN_FILE);

    // Try to load existing token
    if token_path.exists() {
        info!("Loading cached Drive token...");
        if let Ok(token_cache) = load_token(&token_path) {
            if !token_cache.is_expired() {
                info!("Using cached Drive token");
                return Ok(token_cache.access_token);
            }

            // Try to refresh if we have a refresh token
            if let Some(refresh) = token_cache.refresh_token {
                info!("Drive token expired, attempting refresh...");
                let client = create_oauth_client(client_id.clone(), client_secret.clone())?;

                if let Ok(new_token) = refresh_token(&client, &refresh).await {
                    let expires_at = new_token.expires_in()
                        .map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64);

                    let token_cache = TokenCache {
                        access_token: new_token.access_token().secret().clone(),
                        refresh_token: new_token.refresh_token().map(|t| t.secret().clone()).or(Some(refresh)),
                        expires_at,
                    };

                    save_token(&token_path, &token_cache)?;
                    return Ok(token_cache.access_token);
                }
            }
        }
    }

    // Need new authorization
    let (token, _) = authorize_drive(client_id, client_secret).await?;
    Ok(token)
}

/// Get or refresh Drive access token with URL callback for TUI
pub async fn get_drive_token_with_url(client_id: String, client_secret: String, tx: tokio::sync::mpsc::UnboundedSender<String>) -> Result<String> {
    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(DRIVE_TOKEN_FILE);

    // Always attempt fresh authorization for TUI (user can see the URL)
    // Try to load existing token first for immediate success if available
    if token_path.exists() {
        info!("Loading cached Drive token...");
        if let Ok(token_cache) = load_token(&token_path) {
            if !token_cache.is_expired() {
                info!("Using cached Drive token");
                // Send cached success message (popup stays open)
                let _ = tx.send("__DRIVE_AUTH_CACHED_SUCCESS__".to_string());
                return Ok(token_cache.access_token);
            }

            // Try to refresh if we have a refresh token
            if let Some(refresh) = token_cache.refresh_token {
                info!("Drive token expired, attempting refresh...");
                let client = create_oauth_client(client_id.clone(), client_secret.clone())?;

                if let Ok(new_token) = refresh_token(&client, &refresh).await {
                    let expires_at = new_token.expires_in()
                        .map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64);

                    let token_cache = TokenCache {
                        access_token: new_token.access_token().secret().clone(),
                        refresh_token: new_token.refresh_token().map(|t| t.secret().clone()).or(Some(refresh)),
                        expires_at,
                    };

                    save_token(&token_path, &token_cache)?;
                    // Send success message for refreshed tokens
                    let _ = tx.send("__DRIVE_AUTH_REFRESH_SUCCESS__".to_string());
                    return Ok(token_cache.access_token);
                }
            }
        }
    }

    // Need new authorization - send URL first, then proceed
    let (token, auth_url) = authorize_drive(client_id, client_secret).await?;
    // Send the auth URL to the TUI
    let _ = tx.send(format!("__DRIVE_AUTH_URL__:{}", auth_url));
    Ok(token)
}

/// Perform full Drive authorization flow
async fn authorize_drive(client_id: String, client_secret: String) -> Result<(String, String)> {
    let client = create_oauth_client(client_id, client_secret)?;
    let scopes = vec![DRIVE_SCOPE.to_string()];

    let (token, auth_url) = perform_oauth_flow(&client, scopes).await?;

    let expires_at = token.expires_in()
        .map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64);

    let token_cache = TokenCache {
        access_token: token.access_token().secret().clone(),
        refresh_token: token.refresh_token().map(|t| t.secret().clone()),
        expires_at,
    };

    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(DRIVE_TOKEN_FILE);
    save_token(&token_path, &token_cache)?;

    Ok((token_cache.access_token, auth_url))
}

/// Clear Drive token (force re-authorization)
pub fn clear_drive_token() -> Result<()> {
    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(DRIVE_TOKEN_FILE);

    if token_path.exists() {
        fs::remove_file(&token_path)?;
        info!("Drive token cleared");
    } else {
        info!("No Drive token to clear");
    }

    Ok(())
}
