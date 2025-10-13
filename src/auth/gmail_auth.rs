use anyhow::Result;
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
        println!("📧 Loading cached Gmail token...");
        if let Ok(token_cache) = load_token(&token_path) {
            if !token_cache.is_expired() {
                println!("✓ Using cached Gmail token");
                return Ok(token_cache.access_token);
            }

            // Try to refresh if we have a refresh token
            if let Some(refresh) = token_cache.refresh_token {
                println!("🔄 Gmail token expired, attempting refresh...");
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
    println!("📧 Gmail authorization required...");
    authorize_gmail(client_id, client_secret).await
}

/// Perform full Gmail authorization flow
async fn authorize_gmail(client_id: String, client_secret: String) -> Result<String> {
    let client = create_oauth_client(client_id, client_secret)?;
    let scopes = vec![GMAIL_SCOPE.to_string()];

    println!("\n📧 Authorizing Gmail access...");
    let token = perform_oauth_flow(&client, scopes).await?;

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

    println!("✓ Gmail authorized successfully!\n");
    Ok(token_cache.access_token)
}

/// Clear Gmail token (force re-authorization)
pub fn clear_gmail_token() -> Result<()> {
    let config_dir = get_config_dir()?;
    let token_path = config_dir.join(GMAIL_TOKEN_FILE);

    if token_path.exists() {
        fs::remove_file(&token_path)?;
        println!("✓ Gmail token cleared");
    } else {
        println!("ℹ No Gmail token to clear");
    }

    Ok(())
}
