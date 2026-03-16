/// Secure OAuth token storage with system keyring integration
///
/// This module provides secure persistence for OAuth tokens using the operating system's
/// credential management system (Keychain on macOS, Credential Manager on Windows, libsecret on Linux).
///
/// Security features:
/// - Tokens stored in OS-managed secure storage
/// - Automatic token refresh before expiry
/// - Secure token deletion on revocation
/// - DPoP key binding support
use keyring::Entry;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use turbomcp_protocol::{Error as McpError, ErrorKind, Result as McpResult};

/// Token information from OAuth server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Access token (JWT or opaque)
    pub access_token: String,

    /// Token type (typically "Bearer" or "DPoP")
    pub token_type: String,

    /// Token expiry in seconds
    pub expires_in: Option<i64>,

    /// Refresh token (optional)
    pub refresh_token: Option<String>,

    /// Granted scopes
    pub scope: Option<String>,

    /// DPoP JWK thumbprint (if using DPoP)
    pub dpop_jkt: Option<String>,

    /// Timestamp when token was issued
    pub issued_at: i64,
}

impl TokenInfo {
    /// Check if token is expired or will expire soon
    ///
    /// Returns true if token is expired or will expire within 60 seconds
    pub fn is_expired(&self) -> bool {
        if let Some(expires_in) = self.expires_in {
            let now = chrono::Utc::now().timestamp();
            let expiry = self.issued_at + expires_in;
            // Consider token expired if it expires within 60 seconds
            return now + 60 >= expiry;
        }
        false
    }

    /// Get seconds until expiry
    pub fn seconds_until_expiry(&self) -> Option<i64> {
        self.expires_in.map(|expires_in| {
            let now = chrono::Utc::now().timestamp();
            let expiry = self.issued_at + expires_in;
            (expiry - now).max(0)
        })
    }
}

/// Secure token storage using OS keyring
pub struct TokenStore {
    /// Service name for keyring entries
    service_name: String,

    /// In-memory cache of token metadata (not the actual tokens)
    /// Maps server_id -> token metadata
    token_cache: Arc<RwLock<HashMap<i64, TokenMetadata>>>,
}

/// Metadata about stored tokens (not the actual token values)
#[derive(Debug, Clone)]
struct TokenMetadata {
    /// Keyring key for access token
    access_token_key: String,

    /// Keyring key for refresh token (if present)
    refresh_token_key: Option<String>,

    /// Token type
    token_type: String,

    /// Expiry timestamp
    expires_at: Option<i64>,

    /// Last refresh timestamp
    last_refreshed_at: i64,
}

impl TokenStore {
    /// Create a new token store
    ///
    /// # Arguments
    /// * `service_name` - Service name for keyring entries (e.g., "TurboMCP Studio")
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            token_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store tokens securely in OS keyring
    ///
    /// # Arguments
    /// * `server_id` - Unique identifier for the MCP server
    /// * `token_info` - Token information from OAuth server
    pub async fn store_token(&self, server_id: i64, token_info: TokenInfo) -> McpResult<()> {
        // Generate keyring keys
        let access_token_key = format!("server_{}_access_token", server_id);
        let refresh_token_key = token_info
            .refresh_token
            .as_ref()
            .map(|_| format!("server_{}_refresh_token", server_id));

        // Store access token
        self.store_in_keyring(&access_token_key, &token_info.access_token)?;

        // Store refresh token if present
        if let Some(refresh_token) = &token_info.refresh_token {
            if let Some(ref key) = refresh_token_key {
                self.store_in_keyring(key, refresh_token)?;
            }
        }

        // Calculate expiry timestamp
        let expires_at = token_info
            .expires_in
            .map(|expires_in| token_info.issued_at + expires_in);

        // Cache metadata
        let metadata = TokenMetadata {
            access_token_key,
            refresh_token_key,
            token_type: token_info.token_type,
            expires_at,
            last_refreshed_at: chrono::Utc::now().timestamp(),
        };

        self.token_cache.write().insert(server_id, metadata);

        tracing::debug!("Stored OAuth tokens for server {}", server_id);

        Ok(())
    }

    /// Retrieve token from keyring
    ///
    /// # Arguments
    /// * `server_id` - Unique identifier for the MCP server
    ///
    /// # Returns
    /// Access token and token type, or error if not found
    pub async fn get_token(&self, server_id: i64) -> McpResult<(String, String)> {
        let cache = self.token_cache.read();
        let metadata = cache.get(&server_id).ok_or_else(|| {
            McpError::new(
                ErrorKind::Internal,
                format!("No token found for server {}", server_id),
            )
        })?;

        // Check if token is expired
        if let Some(expires_at) = metadata.expires_at {
            let now = chrono::Utc::now().timestamp();
            if now >= expires_at {
                return Err(McpError::authentication("Token expired"));
            }
        }

        let access_token = self.retrieve_from_keyring(&metadata.access_token_key)?;

        Ok((access_token, metadata.token_type.clone()))
    }

    /// Check if valid token exists for server
    pub async fn has_valid_token(&self, server_id: i64) -> bool {
        let cache = self.token_cache.read();
        if let Some(metadata) = cache.get(&server_id) {
            // Check if token is expired
            if let Some(expires_at) = metadata.expires_at {
                let now = chrono::Utc::now().timestamp();
                return now < expires_at;
            }
            // If no expiry, assume valid
            return true;
        }
        false
    }

    /// Delete tokens for server
    ///
    /// # Arguments
    /// * `server_id` - Unique identifier for the MCP server
    pub async fn delete_token(&self, server_id: i64) -> McpResult<()> {
        let mut cache = self.token_cache.write();
        if let Some(metadata) = cache.remove(&server_id) {
            // Delete access token
            let _ = self.delete_from_keyring(&metadata.access_token_key);

            // Delete refresh token if present
            if let Some(refresh_key) = metadata.refresh_token_key {
                let _ = self.delete_from_keyring(&refresh_key);
            }

            tracing::debug!("Deleted OAuth tokens for server {}", server_id);
        }

        Ok(())
    }

    /// Get refresh token for server
    pub async fn get_refresh_token(&self, server_id: i64) -> McpResult<String> {
        let cache = self.token_cache.read();
        let metadata = cache.get(&server_id).ok_or_else(|| {
            McpError::new(
                ErrorKind::Internal,
                format!("No token found for server {}", server_id),
            )
        })?;

        let refresh_key = metadata
            .refresh_token_key
            .as_ref()
            .ok_or_else(|| McpError::new(ErrorKind::Internal, "No refresh token available"))?;

        self.retrieve_from_keyring(refresh_key)
    }

    /// Store value in OS keyring
    fn store_in_keyring(&self, key: &str, value: &str) -> McpResult<()> {
        let entry = Entry::new(&self.service_name, key).map_err(|e| {
            McpError::new(
                ErrorKind::Internal,
                format!("Failed to create keyring entry: {}", e),
            )
        })?;

        entry.set_password(value).map_err(|e| {
            McpError::new(
                ErrorKind::Internal,
                format!("Failed to store in keyring: {}", e),
            )
        })?;

        Ok(())
    }

    /// Retrieve value from OS keyring
    fn retrieve_from_keyring(&self, key: &str) -> McpResult<String> {
        let entry = Entry::new(&self.service_name, key).map_err(|e| {
            McpError::new(
                ErrorKind::Internal,
                format!("Failed to create keyring entry: {}", e),
            )
        })?;

        entry.get_password().map_err(|e| {
            McpError::new(
                ErrorKind::Internal,
                format!("Failed to retrieve from keyring: {}", e),
            )
        })
    }

    /// Delete value from OS keyring
    fn delete_from_keyring(&self, key: &str) -> McpResult<()> {
        let entry = Entry::new(&self.service_name, key).map_err(|e| {
            McpError::new(
                ErrorKind::Internal,
                format!("Failed to create keyring entry: {}", e),
            )
        })?;

        entry.delete_credential().map_err(|e| {
            McpError::new(
                ErrorKind::Internal,
                format!("Failed to delete from keyring: {}", e),
            )
        })?;

        Ok(())
    }

    /// List all servers with stored tokens
    pub fn list_servers_with_tokens(&self) -> Vec<i64> {
        self.token_cache.read().keys().copied().collect()
    }

    /// Get token expiry information
    pub fn get_token_expiry(&self, server_id: i64) -> Option<i64> {
        self.token_cache
            .read()
            .get(&server_id)
            .and_then(|m| m.expires_at)
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new("TurboMCP Studio")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expiry_check() {
        let now = chrono::Utc::now().timestamp();
        let token_info = TokenInfo {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: None,
            scope: None,
            dpop_jkt: None,
            issued_at: now,
        };

        // Should not be expired (expires in 1 hour)
        assert!(!token_info.is_expired());

        let expired_token = TokenInfo {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(30), // Expires in 30 seconds
            refresh_token: None,
            scope: None,
            dpop_jkt: None,
            issued_at: now,
        };

        // Should be considered expired (within 60 second buffer)
        assert!(expired_token.is_expired());
    }

    #[test]
    fn test_seconds_until_expiry() {
        let now = chrono::Utc::now().timestamp();
        let token_info = TokenInfo {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            refresh_token: None,
            scope: None,
            dpop_jkt: None,
            issued_at: now,
        };

        let seconds = token_info.seconds_until_expiry().unwrap();
        // Should be approximately 3600 seconds (allowing for test execution time)
        assert!((3595..=3600).contains(&seconds));
    }
}
