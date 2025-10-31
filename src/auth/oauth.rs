use anyhow::{Context, Result};
use log::{info, warn, error};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenUrl,
};
use oauth2::basic::{BasicClient, BasicTokenType};
use oauth2::reqwest::async_http_client;
use oauth2::StandardTokenResponse;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REDIRECT_URI: &str = "http://localhost:8080";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCache {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

impl TokenCache {
    /// Check if token is expired or close to expiring (within 5 minutes)
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = chrono::Utc::now().timestamp();
            // Consider expired if within 5 minutes of expiration
            now >= (expires_at - 300)
        } else {
            false
        }
    }
}

/// Get the config directory path for token storage
pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("invoice-agent");

    fs::create_dir_all(&config_dir)
        .context("Failed to create config directory")?;

    Ok(config_dir)
}

/// Save token to file
pub fn save_token(token_path: &PathBuf, token: &TokenCache) -> Result<()> {
    let json = serde_json::to_string_pretty(token)
        .context("Failed to serialize token")?;

    fs::write(token_path, json)
        .context("Failed to write token file")?;

    info!("Token saved to {}", token_path.display());
    Ok(())
}

/// Load token from file
pub fn load_token(token_path: &PathBuf) -> Result<TokenCache> {
    let json = fs::read_to_string(token_path)
        .context("Failed to read token file")?;

    let token: TokenCache = serde_json::from_str(&json)
        .context("Failed to parse token file")?;

    Ok(token)
}

/// Create OAuth2 client
pub fn create_oauth_client(
    client_id: String,
    client_secret: String,
) -> Result<BasicClient> {
    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(GOOGLE_AUTH_URL.to_string())?,
        Some(TokenUrl::new(GOOGLE_TOKEN_URL.to_string())?),
    )
    .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_string())?);

    Ok(client)
}

/// Perform OAuth2 authorization flow
pub async fn perform_oauth_flow(
    client: &BasicClient,
    scopes: Vec<String>,
) -> Result<(StandardTokenResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>, String)> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Build authorization URL with scopes
    let mut auth_request = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge);

    for scope in scopes {
        auth_request = auth_request.add_scope(Scope::new(scope));
    }

    let (auth_url, csrf_token) = auth_request.url();

    // Return the auth URL instead of printing it (for TUI compatibility)
    let auth_url_str = auth_url.to_string();

    // Try to open the URL in the default browser
    if let Err(e) = webbrowser::open(&auth_url_str) {
        warn!("Failed to open browser automatically: {}. Please manually open: {}", e, auth_url_str);
    }

    // Start local server to receive callback
    let listener = TcpListener::bind("127.0.0.1:8080")
        .context("Failed to bind to port 8080. Is another instance running?")?;

    // Wait for connection
    let (mut stream, _) = listener.accept()
        .context("Failed to accept connection")?;

    // Read the HTTP request
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Extract code and state from request
    let redirect_url = request_line
        .split_whitespace()
        .nth(1)
        .context("Invalid HTTP request")?;

    let url = url::Url::parse(&format!("http://localhost{}", redirect_url))?;

    let code = url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| AuthorizationCode::new(value.into_owned()))
        .context("Authorization code not found in callback")?;

    let state = url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| CsrfToken::new(value.into_owned()))
        .context("State not found in callback")?;

    // Verify CSRF token
    if state.secret() != csrf_token.secret() {
        anyhow::bail!("CSRF token mismatch");
    }

    // Send success response to browser
    let response = "HTTP/1.1 200 OK\r\n\r\n\
        <html><body>\
        <h1>✓ Authorization successful!</h1>\
        <p>You can close this window and return to the terminal.</p>\
        </body></html>";
    stream.write_all(response.as_bytes())?;

    // Exchange code for token
    let token = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .context("Failed to exchange authorization code for token")?;

    Ok((token, auth_url_str))
}

/// Refresh an expired token
pub async fn refresh_token(
    client: &BasicClient,
    refresh_token: &str,
) -> Result<StandardTokenResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>> {
    info!("Refreshing expired token...");

    let token = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token.to_string()))
        .request_async(async_http_client)
        .await
        .context("Failed to refresh token")?;

    info!("Token refreshed successfully");
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expiration() {
        // Not expired
        let token = TokenCache {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(chrono::Utc::now().timestamp() + 3600),
        };
        assert!(!token.is_expired());

        // Expired
        let token = TokenCache {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(chrono::Utc::now().timestamp() - 100),
        };
        assert!(token.is_expired());

        // Close to expiring (within 5 minutes)
        let token = TokenCache {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(chrono::Utc::now().timestamp() + 200),
        };
        assert!(token.is_expired());
    }

    #[test]
    fn test_config_dir() {
        let config_dir = get_config_dir().unwrap();
        assert!(config_dir.to_string_lossy().contains("invoice-agent"));
    }
}
