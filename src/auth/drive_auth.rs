use anyhow::Result;
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
        println!("📁 Loading cached Drive token...");
        if let Ok(token_cache) = load_token(&token_path) {
            if !token_cache.is_expired() {
                println!("✓ Using cached Drive token");
                return Ok(token_cache.access_token);
            }

            // Try to refresh if we have a refresh token
            if let Some(refresh) = token_cache.refresh_token {
                println!("🔄 Drive token expired, attempting refresh...");
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
    println!("📁 Drive authorization required...");
    authorize_drive(client_id, client_secret).await
}

/// Perform full Drive authorization flow
async fn authorize_drive(client_id: String, client_secret: String) -> Result<String> {
    let client = create_oauth_client(client_id, client_secret)?;
    let scopes = vec![DRIVE_SCOPE.to_string()];

    println!("\n📁 Authorizing Drive access...");
    let token = perform_oauth_flow(&client, scopes).await?;

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

    println!("✓ Drive authorized successfully!\n");
    Ok(token_cache.access_token)
}

/// Clear Drive token (force re-authorization)
pub fn clear_drive_token() -> Result<()> {
    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(DRIVE_TOKEN_FILE);

    if token_path.exists() {
        fs::remove_file(&token_path)?;
        println!("✓ Drive token cleared");
    } else {
        println!("ℹ No Drive token to clear");
    }

    Ok(())
}
