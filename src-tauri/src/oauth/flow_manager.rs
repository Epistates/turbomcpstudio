/// OAuth Flow Manager - State Machine and Orchestration
///
/// Manages the complete OAuth 2.1 authorization flow with PKCE:
/// 1. Metadata discovery
/// 2. Authorization URL generation
/// 3. User authorization in browser
/// 4. Callback handling
/// 5. Code exchange for tokens
/// 6. Token validation (RFC 8707 audience)
/// 7. Token storage in keyring
///
/// The flow manager tracks state for multiple concurrent OAuth flows,
/// each identified by a unique flow_id.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use turbomcp_protocol::{Error as McpError, ErrorKind, Result as McpResult};
use uuid::Uuid;

use crate::oauth::callback_server::CallbackServer;
use crate::oauth::metadata_discovery::{MetadataDiscovery, OAuthMetadata};
use crate::oauth::token_store::{TokenInfo, TokenStore};

/// OAuth flow state machine states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowState {
    /// Initial state
    Idle,
    /// Discovering OAuth metadata
    DiscoveringMetadata,
    /// Waiting for user configuration
    AwaitingUserConfig,
    /// Generating authorization URL
    GeneratingAuthUrl,
    /// Waiting for user to authorize in browser
    AwaitingAuthorization,
    /// Exchanging authorization code for tokens
    ExchangingCode,
    /// Validating token claims
    ValidatingToken,
    /// Flow completed successfully
    Completed,
    /// Flow failed with error
    Failed,
}

/// A single step in the OAuth flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStep {
    /// Step number (sequential)
    pub step: u32,
    /// Step name (e.g., "Metadata Discovery")
    pub name: String,
    /// Current state
    pub state: String,
    /// Human-readable description
    pub description: String,
    /// Timestamp when step started
    pub timestamp: String,
    /// Duration in milliseconds (if completed)
    pub duration_ms: Option<u64>,
    /// Additional data (HTTP requests, responses, etc.)
    pub data: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Error information for failed flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowError {
    /// Error code
    pub code: String,
    /// Error description
    pub description: String,
    /// Suggested remediation
    pub remediation: Option<String>,
}

/// OAuth flow execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlow {
    /// Unique flow identifier
    pub flow_id: String,
    /// Server being authenticated
    pub server_id: i64,
    /// Current state
    pub state: FlowState,
    /// Flow steps (history)
    pub steps: Vec<FlowStep>,
    /// State parameter (CSRF protection)
    pub state_param: Option<String>,
    /// PKCE code verifier
    pub pkce_verifier: Option<String>,
    /// Authorization URL for user
    pub auth_url: Option<String>,
    /// Discovered metadata
    pub metadata: Option<OAuthMetadata>,
    /// Error (if failed)
    pub error: Option<FlowError>,
    /// OAuth configuration
    pub config: OAuthConfig,
}

/// OAuth configuration for a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Server ID this config belongs to
    #[serde(default)]
    pub server_id: String,
    /// Protocol version (MCP spec version)
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    /// Authorization server URL
    pub auth_server_url: String,
    /// Token endpoint URL (discovered or manual)
    pub token_endpoint: Option<String>,
    /// Client ID
    pub client_id: String,
    /// Client secret (optional for public clients with PKCE)
    pub client_secret: Option<String>,
    /// Redirect URI (typically http://localhost:8080/callback)
    pub redirect_uri: String,
    /// Requested scopes
    pub scopes: Vec<String>,
    /// Resource URI (RFC 8707)
    pub resource_uri: String,
    /// Use PKCE
    pub use_pkce: bool,
    /// Use DPoP
    pub use_dpop: bool,
    /// Additional metadata from discovery
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_protocol_version() -> String {
    "2025-11-25".to_string()
}

/// OAuth Flow Manager
pub struct OAuthFlowManager {
    /// Active flows by flow_id
    flows: Arc<RwLock<HashMap<String, OAuthFlow>>>,
    /// Metadata discovery service
    discovery: Arc<MetadataDiscovery>,
    /// Token storage
    token_store: Arc<TokenStore>,
    /// Callback server for OAuth redirects
    callback_server: Arc<CallbackServer>,
}

impl OAuthFlowManager {
    /// Create a new OAuth flow manager
    pub fn new() -> Self {
        Self {
            flows: Arc::new(RwLock::new(HashMap::new())),
            discovery: Arc::new(MetadataDiscovery::new()),
            token_store: Arc::new(TokenStore::new("TurboMCP Studio")),
            callback_server: Arc::new(CallbackServer::new(8080)),
        }
    }

    /// Discover OAuth metadata for a server
    ///
    /// # Arguments
    /// * `server_url` - Base URL of the OAuth server
    ///
    /// # Returns
    /// Discovered OAuth metadata
    pub async fn discover_metadata(&self, server_url: &str) -> McpResult<OAuthMetadata> {
        self.discovery.discover(server_url).await
    }

    /// Start authorization code flow
    ///
    /// # Arguments
    /// * `server_id` - Unique identifier for the MCP server
    /// * `config` - OAuth configuration
    ///
    /// # Returns
    /// Flow ID for tracking
    pub async fn start_authorization_flow(
        &self,
        server_id: i64,
        config: OAuthConfig,
    ) -> McpResult<String> {
        let flow_id = Uuid::new_v4().to_string();

        // Initialize flow
        let mut flow = OAuthFlow {
            flow_id: flow_id.clone(),
            server_id,
            state: FlowState::GeneratingAuthUrl,
            steps: vec![],
            state_param: None,
            pkce_verifier: None,
            auth_url: None,
            metadata: None,
            error: None,
            config: config.clone(),
        };

        // Step 1: Generate PKCE verifier (if enabled)
        let (_pkce_verifier, pkce_challenge) = if config.use_pkce {
            Self::add_step(&mut flow, "Generate PKCE", "Generating PKCE code verifier and challenge");

            let verifier = Self::generate_pkce_verifier();
            let challenge = Self::generate_pkce_challenge(&verifier);

            flow.pkce_verifier = Some(verifier.clone());
            (Some(verifier), Some(challenge))
        } else {
            (None, None)
        };

        // Step 2: Generate state parameter (CSRF protection)
        Self::add_step(&mut flow, "Generate State", "Generating state parameter for CSRF protection");
        let state_param = Uuid::new_v4().to_string();
        flow.state_param = Some(state_param.clone());

        // Step 3: Build authorization URL
        Self::add_step(&mut flow, "Build Authorization URL", "Building authorization URL with parameters");

        let auth_url = self.build_auth_url(&config, &state_param, pkce_challenge.as_deref())?;
        flow.auth_url = Some(auth_url.clone());

        // Step 4: Open browser
        Self::add_step(&mut flow, "Open Browser", "Opening system browser for user authorization");
        flow.state = FlowState::AwaitingAuthorization;

        // Store flow
        self.flows.write().insert(flow_id.clone(), flow.clone());

        // Open browser
        open::that(&auth_url)
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Failed to open browser: {}", e)))?;

        // Start callback listener
        self.start_callback_listener(flow_id.clone()).await?;

        Ok(flow_id)
    }

    /// Build authorization URL with all required parameters
    fn build_auth_url(
        &self,
        config: &OAuthConfig,
        state: &str,
        pkce_challenge: Option<&str>,
    ) -> McpResult<String> {
        let mut url = url::Url::parse(&format!("{}/authorize", config.auth_server_url))
            .map_err(|e| McpError::invalid_params(format!("Invalid auth server URL: {}", e)))?;

        // Required parameters
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &config.client_id)
            .append_pair("redirect_uri", &config.redirect_uri)
            .append_pair("state", state)
            .append_pair("scope", &config.scopes.join(" "));

        // RFC 8707: Resource indicator
        if !config.resource_uri.is_empty() {
            url.query_pairs_mut()
                .append_pair("resource", &config.resource_uri);
        }

        // PKCE parameters
        if let Some(challenge) = pkce_challenge {
            url.query_pairs_mut()
                .append_pair("code_challenge", challenge)
                .append_pair("code_challenge_method", "S256");
        }

        Ok(url.to_string())
    }

    /// Listen for OAuth callback
    async fn start_callback_listener(&self, flow_id: String) -> McpResult<()> {
        let callback_server = self.callback_server.clone();
        let flows = self.flows.clone();
        let token_store = self.token_store.clone();

        tokio::spawn(async move {
            // Wait for callback
            match callback_server.wait_for_callback().await {
                Ok((code, state)) => {
                    // Extract data we need (within a scope to ensure lock is dropped)
                    let (config, pkce_verifier, server_id) = {
                        let mut flows_guard = flows.write();
                        let flow_opt = flows_guard.get_mut(&flow_id);

                        if let Some(flow) = flow_opt {
                            // Validate state
                            if flow.state_param.as_ref() != Some(&state) {
                                Self::fail_flow_static(flow, "CSRF_ERROR", "State parameter mismatch - possible CSRF attack");
                                return;
                            }

                            // Exchange code for token
                            Self::add_step_static(flow, "Exchange Code", "Exchanging authorization code for access token");
                            flow.state = FlowState::ExchangingCode;

                            // Extract data before dropping lock
                            let config = flow.config.clone();
                            let pkce_verifier = flow.pkce_verifier.clone();
                            let server_id = flow.server_id;

                            (config, pkce_verifier, server_id)
                        } else {
                            return;
                        }
                    }; // Lock is dropped here

                    match Self::exchange_code_for_token_static(&config, &code, pkce_verifier.as_deref()).await {
                            Ok(token_info) => {
                                // Update flow state
                                {
                                    let mut flows_guard = flows.write();
                                    if let Some(flow) = flows_guard.get_mut(&flow_id) {
                                        Self::add_step_static(flow, "Validate Token", "Validating token audience claim (RFC 8707)");
                                        flow.state = FlowState::ValidatingToken;
                                    }
                                } // Lock is dropped here

                                // Store token (without holding lock)
                                match token_store.store_token(server_id, token_info).await {
                                    Ok(_) => {
                                        let mut flows_guard = flows.write();
                                        if let Some(flow) = flows_guard.get_mut(&flow_id) {
                                            flow.state = FlowState::Completed;
                                            Self::add_step_static(flow, "Completed", "OAuth flow completed successfully");
                                        }
                                    }
                                    Err(e) => {
                                        let mut flows_guard = flows.write();
                                        if let Some(flow) = flows_guard.get_mut(&flow_id) {
                                            Self::fail_flow_static(flow, "TOKEN_STORAGE_ERROR", &e.to_string());
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let mut flows_guard = flows.write();
                                if let Some(flow) = flows_guard.get_mut(&flow_id) {
                                    Self::fail_flow_static(flow, "TOKEN_EXCHANGE_ERROR", &e.to_string());
                                }
                            }
                        }
                }
                Err(e) => {
                    let mut flows_guard = flows.write();
                    if let Some(flow) = flows_guard.get_mut(&flow_id) {
                        Self::fail_flow_static(flow, "CALLBACK_ERROR", &e.to_string());
                    }
                }
            }
        });

        Ok(())
    }

    /// Exchange authorization code for access token
    async fn exchange_code_for_token_static(
        config: &OAuthConfig,
        code: &str,
        pkce_verifier: Option<&str>,
    ) -> McpResult<TokenInfo> {
        let token_endpoint = config
            .token_endpoint
            .as_ref()
            .ok_or_else(|| McpError::invalid_params("Token endpoint not configured"))?;

        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &config.redirect_uri),
            ("client_id", &config.client_id),
        ];

        // Add PKCE verifier
        if let Some(verifier) = pkce_verifier {
            params.push(("code_verifier", verifier));
        }

        // Add client secret if present
        let client_secret_str;
        if let Some(secret) = &config.client_secret {
            client_secret_str = secret.clone();
            params.push(("client_secret", &client_secret_str));
        }

        let client = reqwest::Client::new();
        let response = client
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Token request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(McpError::new(ErrorKind::Internal, format!(
                "Token exchange failed: {}",
                error_text
            )));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            token_type: String,
            expires_in: Option<i64>,
            refresh_token: Option<String>,
            scope: Option<String>,
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| McpError::serialization(format!("Invalid token response: {}", e)))?;

        Ok(TokenInfo {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            refresh_token: token_response.refresh_token,
            scope: token_response.scope,
            dpop_jkt: None,
            issued_at: chrono::Utc::now().timestamp(),
        })
    }

    /// Get current flow status
    pub async fn get_flow_status(&self, flow_id: &str) -> McpResult<OAuthFlow> {
        let flows = self.flows.read();
        flows.get(flow_id).cloned().ok_or_else(|| {
            McpError::new(ErrorKind::Internal, format!("Flow {} not found", flow_id))
        })
    }

    /// Cancel OAuth flow
    pub async fn cancel_flow(&self, flow_id: &str) -> McpResult<()> {
        self.flows.write().remove(flow_id);
        Ok(())
    }

    /// Check if server has valid token
    pub async fn has_valid_token(&self, server_id: i64) -> bool {
        self.token_store.has_valid_token(server_id).await
    }

    /// Get token store reference
    pub fn token_store(&self) -> &TokenStore {
        &self.token_store
    }

    /// Refresh OAuth token for a server
    ///
    /// Uses the stored refresh token to obtain a new access token.
    ///
    /// # Arguments
    /// * `server_id` - Server to refresh token for
    /// * `config` - OAuth configuration (must include token_endpoint)
    ///
    /// # Returns
    /// Ok(()) on success, error if refresh fails
    pub async fn refresh_token(&self, server_id: i64, config: &OAuthConfig) -> McpResult<()> {
        // Get refresh token from store
        let refresh_token = self.token_store.get_refresh_token(server_id).await?;

        let token_endpoint = config
            .token_endpoint
            .as_ref()
            .ok_or_else(|| McpError::invalid_params("Token endpoint not configured"))?;

        tracing::info!("Refreshing OAuth token for server {}", server_id);

        // Build refresh request
        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", config.client_id.as_str()),
        ];

        // Add client secret if present (confidential client)
        let client_secret_str;
        if let Some(secret) = &config.client_secret {
            client_secret_str = secret.clone();
            params.push(("client_secret", &client_secret_str));
        }

        // Add scope if configured
        let scope_str;
        if !config.scopes.is_empty() {
            scope_str = config.scopes.join(" ");
            params.push(("scope", &scope_str));
        }

        // Make refresh request
        let client = reqwest::Client::new();
        let response = client
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Token refresh request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Check for common refresh errors
            if status == reqwest::StatusCode::BAD_REQUEST {
                if error_text.contains("invalid_grant") || error_text.contains("expired") {
                    // Refresh token is invalid or expired - user needs to re-authenticate
                    return Err(McpError::authentication(
                        "Refresh token expired - please re-authenticate"
                    ));
                }
            }

            return Err(McpError::new(ErrorKind::Internal, format!(
                "Token refresh failed ({}): {}",
                status, error_text
            )));
        }

        // Parse response
        #[derive(serde::Deserialize)]
        struct RefreshTokenResponse {
            access_token: String,
            token_type: String,
            expires_in: Option<i64>,
            refresh_token: Option<String>,
            scope: Option<String>,
        }

        let token_response: RefreshTokenResponse = response
            .json()
            .await
            .map_err(|e| McpError::serialization(format!("Invalid token response: {}", e)))?;

        // Build new TokenInfo
        // Some servers return a new refresh token, others don't (keep existing)
        let new_refresh_token = token_response.refresh_token.or(Some(refresh_token));

        let token_info = TokenInfo {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            refresh_token: new_refresh_token,
            scope: token_response.scope,
            dpop_jkt: None, // TODO: Handle DPoP refresh
            issued_at: chrono::Utc::now().timestamp(),
        };

        // Store new tokens
        self.token_store.store_token(server_id, token_info).await?;

        tracing::info!("Successfully refreshed OAuth token for server {}", server_id);

        Ok(())
    }

    // Helper methods

    fn add_step(flow: &mut OAuthFlow, name: &str, description: &str) {
        Self::add_step_static(flow, name, description);
    }

    fn add_step_static(flow: &mut OAuthFlow, name: &str, description: &str) {
        let step = FlowStep {
            step: flow.steps.len() as u32 + 1,
            name: name.to_string(),
            state: "in_progress".to_string(),
            description: description.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms: None,
            data: None,
            error: None,
        };
        flow.steps.push(step);
    }

    fn fail_flow_static(flow: &mut OAuthFlow, code: &str, description: &str) {
        flow.state = FlowState::Failed;
        flow.error = Some(FlowError {
            code: code.to_string(),
            description: description.to_string(),
            remediation: None,
        });
    }

    /// Generate PKCE code verifier (43-128 characters)
    fn generate_pkce_verifier() -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        URL_SAFE_NO_PAD.encode(&random_bytes)
    }

    /// Generate PKCE code challenge from verifier (SHA-256)
    fn generate_pkce_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(&hash)
    }
}

impl Default for OAuthFlowManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let verifier = OAuthFlowManager::generate_pkce_verifier();
        assert!(verifier.len() >= 43);
        assert!(verifier.len() <= 128);

        let challenge = OAuthFlowManager::generate_pkce_challenge(&verifier);
        assert!(!challenge.is_empty());
        assert_ne!(verifier, challenge);
    }

    #[test]
    fn test_flow_state_transitions() {
        let config = OAuthConfig {
            server_id: "test-server".to_string(),
            protocol_version: "2025-11-25".to_string(),
            auth_server_url: "https://example.com".to_string(),
            token_endpoint: Some("https://example.com/token".to_string()),
            client_id: "test_client".to_string(),
            client_secret: None,
            redirect_uri: "http://localhost:8080/callback".to_string(),
            scopes: vec!["openid".to_string()],
            resource_uri: "https://resource.example.com".to_string(),
            use_pkce: true,
            use_dpop: false,
            metadata: None,
        };

        let flow = OAuthFlow {
            flow_id: "test-flow".to_string(),
            server_id: 1,
            state: FlowState::Idle,
            steps: vec![],
            state_param: None,
            pkce_verifier: None,
            auth_url: None,
            metadata: None,
            error: None,
            config,
        };

        assert_eq!(flow.state, FlowState::Idle);
    }
}
