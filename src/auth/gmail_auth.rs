use anyhow::Result;
use log::info;
use oauth2::TokenResponse;
use super::oauth::{
    TokenCache, create_oauth_client, get_config_dir, load_token, save_token,
    perform_oauth_flow, refresh_token,
};
use std::fs;

const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
const GMAIL_TOKEN_FILE: &str = "gmail_token.json";

/// Get or refresh Gmail access token
pub async fn get_gmail_token(client_id: String, client_secret: String) -> Result<String> {
    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(GMAIL_TOKEN_FILE);

    // Try to load existing token
    if token_path.exists() {
        info!("Loading cached Gmail token...");
        if let Ok(token_cache) = load_token(&token_path) {
            if !token_cache.is_expired() {
                info!("Using cached Gmail token");
                return Ok(token_cache.access_token);
            }

            // Try to refresh if we have a refresh token
            if let Some(refresh) = token_cache.refresh_token {
                info!("Gmail token expired, attempting refresh...");
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
    let (token, _) = authorize_gmail(client_id, client_secret, None).await?;
    Ok(token)
}

/// Get or refresh Gmail access token with URL callback for TUI
pub async fn get_gmail_token_with_url(client_id: String, client_secret: String, tx: tokio::sync::mpsc::UnboundedSender<String>) -> Result<String> {
    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(GMAIL_TOKEN_FILE);

    // For TUI, always show the OAuth flow to give user control
    // But try to use cached tokens if available and valid
    if token_path.exists() {
        info!("Loading cached Gmail token...");
        if let Ok(token_cache) = load_token(&token_path) {
            if !token_cache.is_expired() {
                info!("Using cached Gmail token");
                // For TUI, send cached success message (popup stays open)
                let _ = tx.send("__GMAIL_AUTH_CACHED_SUCCESS__".to_string());
                return Ok(token_cache.access_token);
            }

            // Try to refresh if we have a refresh token
            if let Some(refresh) = token_cache.refresh_token {
                info!("Gmail token expired, attempting refresh...");
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
                    let _ = tx.send("__GMAIL_AUTH_REFRESH_SUCCESS__".to_string());
                    return Ok(token_cache.access_token);
                }
            }
        }
    }

    // Need new authorization - URL will be sent via channel from perform_oauth_flow
    let (token, _auth_url) = authorize_gmail(client_id, client_secret, Some(tx)).await?;
    Ok(token)
}

/// Perform full Gmail authorization flow
async fn authorize_gmail(client_id: String, client_secret: String, tx: Option<tokio::sync::mpsc::UnboundedSender<String>>) -> Result<(String, String)> {
    let client = create_oauth_client(client_id, client_secret)?;
    let scopes = vec![GMAIL_SCOPE.to_string()];

    let sender_with_prefix = tx.map(|sender| (sender, "GMAIL_"));
    let (token, auth_url) = perform_oauth_flow(&client, scopes, sender_with_prefix).await?;

    let expires_at = token.expires_in()
        .map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64);

    let token_cache = TokenCache {
        access_token: token.access_token().secret().clone(),
        refresh_token: token.refresh_token().map(|t| t.secret().clone()),
        expires_at,
    };

    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(GMAIL_TOKEN_FILE);
    save_token(&token_path, &token_cache)?;

    Ok((token_cache.access_token, auth_url))
}

/// Clear Gmail token (force re-authorization)
pub fn clear_gmail_token() -> Result<()> {
    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(GMAIL_TOKEN_FILE);

    if token_path.exists() {
        fs::remove_file(&token_path)?;
        info!("Gmail token cleared");
    } else {
        info!("No Gmail token to clear");
    }

    Ok(())
}
