/// OAuth 2.1 Tauri Command Handlers
///
/// Provides frontend API for OAuth visual debugger functionality:
/// - Metadata discovery
/// - Authorization flow management
/// - Token inspection and management
/// - Configuration persistence and import/export
use crate::error::{McpResult, McpStudioError};
use crate::oauth::flow_manager::{OAuthConfig, OAuthFlow};
use crate::oauth::metadata_discovery::OAuthMetadata;
use crate::oauth::provider_templates::{
    get_provider_template, get_provider_templates, validate_manual_config, OAuthProviderTemplate,
};
use crate::AppState;
use serde_json::Value;
use tauri::State;

/// Wrapper to access OAuthFlowManager from AppState
/// This will be implemented after AppState integration
#[derive(Clone)]
pub struct OAuthState {
    pub flow_manager: std::sync::Arc<crate::oauth::OAuthFlowManager>,
}

/// Discover OAuth metadata for a server
///
/// # Arguments
/// * `server_url` - Base URL of the OAuth server or MCP server
/// * `protocol_version` - MCP OAuth protocol version (e.g., "2025-06-18")
///
/// # Returns
/// Discovered OAuth metadata (RFC 8414 + RFC 9728)
#[tauri::command]
pub async fn discover_oauth_metadata(
    server_url: String,
    protocol_version: String,
    state: State<'_, OAuthState>,
) -> McpResult<OAuthMetadata> {
    tracing::info!(
        "Discovering OAuth metadata for {} (protocol: {})",
        server_url,
        protocol_version
    );

    state
        .flow_manager
        .discover_metadata(&server_url)
        .await
        .map_err(|e| McpStudioError::ConfigError(e.to_string()))
}

/// Start OAuth authorization flow
///
/// Opens browser for user authorization and tracks flow progress
///
/// # Arguments
/// * `server_id` - Unique identifier for the MCP server
/// * `config` - OAuth configuration
///
/// # Returns
/// Flow ID for tracking progress
#[tauri::command]
pub async fn start_oauth_authorization_flow(
    server_id: i64,
    config: OAuthConfig,
    state: State<'_, OAuthState>,
) -> McpResult<String> {
    tracing::info!("Starting OAuth authorization flow for server {}", server_id);

    state
        .flow_manager
        .start_authorization_flow(server_id, config)
        .await
        .map_err(|e| McpStudioError::ConfigError(e.to_string()))
}

/// Get current OAuth flow status
///
/// # Arguments
/// * `flow_id` - Unique flow identifier
///
/// # Returns
/// Current flow state with steps and metadata
#[tauri::command]
pub async fn get_oauth_flow_status(
    flow_id: String,
    state: State<'_, OAuthState>,
) -> McpResult<OAuthFlow> {
    state
        .flow_manager
        .get_flow_status(&flow_id)
        .await
        .map_err(|e| McpStudioError::ConfigError(e.to_string()))
}

/// Cancel an active OAuth flow
///
/// # Arguments
/// * `flow_id` - Unique flow identifier
#[tauri::command]
pub async fn cancel_oauth_flow(flow_id: String, state: State<'_, OAuthState>) -> McpResult<()> {
    tracing::info!("Cancelling OAuth flow: {}", flow_id);

    state
        .flow_manager
        .cancel_flow(&flow_id)
        .await
        .map_err(|e| McpStudioError::ConfigError(e.to_string()))
}

/// Decode JWT token for inspection
///
/// Decodes JWT token without validation (for inspection purposes only)
///
/// # Arguments
/// * `token` - JWT token string
///
/// # Returns
/// Decoded JWT header and claims
#[tauri::command]
pub async fn decode_jwt_token(token: String) -> McpResult<Value> {
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    let mut validation = Validation::new(Algorithm::RS256);
    validation.insecure_disable_signature_validation();

    let decoded = decode::<Value>(&token, &DecodingKey::from_secret(&[]), &validation)
        .map_err(|e| McpStudioError::ConfigError(format!("Failed to decode JWT: {}", e)))?;

    Ok(serde_json::json!({
        "header": decoded.header,
        "claims": decoded.claims,
    }))
}

/// Refresh OAuth token for a server
///
/// Uses the stored refresh token to obtain a new access token.
/// Requires the OAuth configuration to know the token endpoint.
///
/// # Arguments
/// * `server_id` - Server to refresh token for
/// * `config` - OAuth configuration with token_endpoint
///
/// # Returns
/// Ok(()) on success, or error if refresh fails
#[tauri::command]
pub async fn refresh_oauth_token(
    server_id: i64,
    config: OAuthConfig,
    state: State<'_, OAuthState>,
) -> McpResult<()> {
    tracing::info!("Refreshing OAuth token for server {}", server_id);

    state
        .flow_manager
        .refresh_token(server_id, &config)
        .await
        .map_err(|e| McpStudioError::AuthError(e.to_string()))
}

/// Revoke OAuth token for a server
///
/// Removes token from secure storage
///
/// # Arguments
/// * `server_id` - Server to revoke token for
#[tauri::command]
pub async fn revoke_oauth_token(server_id: i64, state: State<'_, OAuthState>) -> McpResult<()> {
    tracing::info!("Revoking OAuth token for server {}", server_id);

    state
        .flow_manager
        .token_store()
        .delete_token(server_id)
        .await
        .map_err(|e| McpStudioError::ConfigError(e.to_string()))
}

/// Export OAuth configuration
///
/// Exports configuration in a portable JSON format compatible with
/// Claude Desktop and other MCP clients.
///
/// # Arguments
/// * `server_id` - Server to export configuration for (string ID)
///
/// # Returns
/// JSON string containing the OAuth configuration
#[tauri::command]
pub async fn export_oauth_config(
    server_id: String,
    app_state: State<'_, AppState>,
) -> McpResult<String> {
    tracing::info!("Exporting OAuth config for server {}", server_id);

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        // Load config from database
        let config = database
            .get_oauth_config(&server_id)
            .await
            .map_err(|e| {
                McpStudioError::ConfigError(format!("Failed to load OAuth config: {}", e))
            })?
            .ok_or_else(|| {
                McpStudioError::ConfigError(format!(
                    "No OAuth config found for server {}",
                    server_id
                ))
            })?;

        // Create export format (Claude Desktop compatible).
        // client_secret is intentionally replaced with a placeholder: secrets
        // must never be written to export files — the user must enter the
        // secret manually after importing the config into another environment.
        let export_data = serde_json::json!({
            "oauth": {
                "client_id": config.client_id,
                "client_secret": "<REDACTED - enter manually>",
                "auth_url": config.auth_server_url,
                "token_url": config.token_endpoint,
                "redirect_uri": config.redirect_uri,
                "scopes": config.scopes,
                "resource_uri": config.resource_uri,
                "pkce": config.use_pkce,
                "dpop": config.use_dpop
            },
            "metadata": {
                "protocol_version": config.protocol_version,
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "source": "TurboMCP Studio"
            }
        });

        serde_json::to_string_pretty(&export_data)
            .map_err(|e| McpStudioError::ConfigError(format!("Failed to serialize config: {}", e)))
    } else {
        Err(McpStudioError::ConfigError(
            "Database not yet initialized".to_string(),
        ))
    }
}

/// Import OAuth configuration
///
/// Imports configuration from Claude Desktop or other MCP clients.
/// Supports the standard MCP OAuth configuration format.
///
/// # Arguments
/// * `config_json` - Configuration JSON string
/// * `server_id` - Server ID to associate this config with
///
/// # Returns
/// Server ID of the imported configuration
#[tauri::command]
pub async fn import_oauth_config(
    config_json: String,
    server_id: String,
    app_state: State<'_, AppState>,
) -> McpResult<String> {
    tracing::info!("Importing OAuth configuration for server {}", server_id);

    // Parse the import format
    let import_data: serde_json::Value = serde_json::from_str(&config_json)
        .map_err(|e| McpStudioError::ConfigError(format!("Invalid JSON: {}", e)))?;

    // Extract OAuth config (support both flat and nested formats)
    let oauth_data = import_data.get("oauth").unwrap_or(&import_data);

    // Build OAuthConfig from imported data
    let config = OAuthConfig {
        server_id: server_id.clone(),
        protocol_version: import_data
            .get("metadata")
            .and_then(|m| m.get("protocol_version"))
            .and_then(|v| v.as_str())
            .unwrap_or("2025-11-25")
            .to_string(),
        auth_server_url: oauth_data
            .get("auth_url")
            .or_else(|| oauth_data.get("authorization_endpoint"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpStudioError::ConfigError("Missing auth_url".to_string()))?
            .to_string(),
        token_endpoint: oauth_data
            .get("token_url")
            .or_else(|| oauth_data.get("token_endpoint"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        client_id: oauth_data
            .get("client_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpStudioError::ConfigError("Missing client_id".to_string()))?
            .to_string(),
        client_secret: oauth_data
            .get("client_secret")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        redirect_uri: oauth_data
            .get("redirect_uri")
            .and_then(|v| v.as_str())
            .unwrap_or("http://localhost:8080/callback")
            .to_string(),
        scopes: oauth_data
            .get("scopes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default(),
        resource_uri: oauth_data
            .get("resource_uri")
            .or_else(|| oauth_data.get("resource"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        use_pkce: oauth_data
            .get("pkce")
            .and_then(|v| v.as_bool())
            .unwrap_or(true), // PKCE is recommended
        use_dpop: oauth_data
            .get("dpop")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        metadata: import_data.get("metadata").cloned(),
    };

    // Save to database
    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database.save_oauth_config(&config).await.map_err(|e| {
            McpStudioError::ConfigError(format!("Failed to save OAuth config: {}", e))
        })?;

        tracing::info!(
            "Successfully imported OAuth config for server {}",
            server_id
        );
        Ok(server_id)
    } else {
        Err(McpStudioError::ConfigError(
            "Database not yet initialized".to_string(),
        ))
    }
}

/// Check if server has valid OAuth token
///
/// # Arguments
/// * `server_id` - Server to check
///
/// # Returns
/// True if valid token exists
#[tauri::command]
pub async fn has_valid_oauth_token(
    server_id: i64,
    state: State<'_, OAuthState>,
) -> McpResult<bool> {
    Ok(state.flow_manager.has_valid_token(server_id).await)
}

/// Get OAuth token for server
///
/// # Arguments
/// * `server_id` - Server to get token for
///
/// # Returns
/// Tuple of (access_token, token_type)
#[tauri::command]
pub async fn get_oauth_token(
    server_id: i64,
    state: State<'_, OAuthState>,
) -> McpResult<(String, String)> {
    state
        .flow_manager
        .token_store()
        .get_token(server_id)
        .await
        .map_err(|e| McpStudioError::ConfigError(e.to_string()))
}

/// Get all available OAuth provider templates
///
/// Returns pre-configured templates for popular OAuth providers
/// (GitHub, Google, Microsoft, etc.)
///
/// # Returns
/// List of provider templates with endpoints and configuration
#[tauri::command]
pub async fn get_oauth_provider_templates() -> McpResult<Vec<OAuthProviderTemplate>> {
    tracing::info!("Getting OAuth provider templates");
    Ok(get_provider_templates())
}

/// Get a specific OAuth provider template by ID
///
/// # Arguments
/// * `provider_id` - Provider identifier (e.g., "github", "google")
///
/// # Returns
/// Provider template if found
#[tauri::command]
pub async fn get_oauth_provider_template(provider_id: String) -> McpResult<OAuthProviderTemplate> {
    tracing::info!("Getting OAuth provider template: {}", provider_id);

    get_provider_template(&provider_id).ok_or_else(|| {
        McpStudioError::ConfigError(format!("Provider template not found: {}", provider_id))
    })
}

/// Validate manual OAuth configuration
///
/// Validates that authorization and token endpoint URLs are valid
///
/// # Arguments
/// * `authorization_endpoint` - Authorization endpoint URL
/// * `token_endpoint` - Token endpoint URL
///
/// # Returns
/// Ok if configuration is valid, error message otherwise
#[tauri::command]
pub async fn validate_oauth_manual_config(
    authorization_endpoint: String,
    token_endpoint: String,
) -> McpResult<()> {
    tracing::info!("Validating manual OAuth configuration");

    validate_manual_config(&authorization_endpoint, &token_endpoint)
        .map_err(McpStudioError::ConfigError)
}

// =============================================================================
// OAuth Configuration Persistence (Database CRUD)
// =============================================================================

/// Save OAuth configuration to database
///
/// # Arguments
/// * `config` - OAuth configuration to save
///
/// # Returns
/// Server ID of the saved configuration
#[tauri::command]
pub async fn save_oauth_config(
    config: OAuthConfig,
    app_state: State<'_, AppState>,
) -> McpResult<String> {
    tracing::info!("Saving OAuth config for server: {}", config.server_id);

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database
            .save_oauth_config(&config)
            .await
            .map_err(|e| McpStudioError::ConfigError(format!("Failed to save OAuth config: {}", e)))
    } else {
        Err(McpStudioError::ConfigError(
            "Database not yet initialized".to_string(),
        ))
    }
}

/// Load OAuth configuration from database
///
/// # Arguments
/// * `server_id` - Server identifier
///
/// # Returns
/// OAuth configuration if found
#[tauri::command]
pub async fn load_oauth_config(
    server_id: String,
    app_state: State<'_, AppState>,
) -> McpResult<Option<OAuthConfig>> {
    tracing::info!("Loading OAuth config for server: {}", server_id);

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database
            .get_oauth_config(&server_id)
            .await
            .map_err(|e| McpStudioError::ConfigError(format!("Failed to load OAuth config: {}", e)))
    } else {
        Err(McpStudioError::ConfigError(
            "Database not yet initialized".to_string(),
        ))
    }
}

/// Update OAuth configuration in database
///
/// # Arguments
/// * `server_id` - Server identifier
/// * `config` - Updated OAuth configuration
#[tauri::command]
pub async fn update_oauth_config(
    server_id: String,
    config: OAuthConfig,
    app_state: State<'_, AppState>,
) -> McpResult<()> {
    tracing::info!("Updating OAuth config for server: {}", server_id);

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database
            .update_oauth_config(&server_id, &config)
            .await
            .map_err(|e| {
                McpStudioError::ConfigError(format!("Failed to update OAuth config: {}", e))
            })
    } else {
        Err(McpStudioError::ConfigError(
            "Database not yet initialized".to_string(),
        ))
    }
}

/// Delete OAuth configuration from database
///
/// # Arguments
/// * `server_id` - Server identifier
#[tauri::command]
pub async fn delete_oauth_config(
    server_id: String,
    app_state: State<'_, AppState>,
) -> McpResult<()> {
    tracing::info!("Deleting OAuth config for server: {}", server_id);

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database.delete_oauth_config(&server_id).await.map_err(|e| {
            McpStudioError::ConfigError(format!("Failed to delete OAuth config: {}", e))
        })
    } else {
        Err(McpStudioError::ConfigError(
            "Database not yet initialized".to_string(),
        ))
    }
}

/// List all OAuth configurations from database
///
/// # Returns
/// List of all OAuth configurations
#[tauri::command]
pub async fn list_oauth_configs(app_state: State<'_, AppState>) -> McpResult<Vec<OAuthConfig>> {
    tracing::info!("Listing all OAuth configs");

    let db_lock = app_state.database.read().await;
    if let Some(database) = db_lock.as_ref() {
        database.list_oauth_configs().await.map_err(|e| {
            McpStudioError::ConfigError(format!("Failed to list OAuth configs: {}", e))
        })
    } else {
        Err(McpStudioError::ConfigError(
            "Database not yet initialized".to_string(),
        ))
    }
}
