/// OAuth Authorization Server Metadata Discovery
///
/// Implements:
/// - RFC 8414: OAuth 2.0 Authorization Server Metadata
/// - RFC 9728: OAuth 2.0 Protected Resource Metadata
///
/// These RFCs are required by the MCP specification for OAuth-protected servers.
/// They allow automatic discovery of OAuth configuration from a well-known URL.
///
/// Discovery URLs:
/// - Authorization Server: https://example.com/.well-known/oauth-authorization-server
/// - Protected Resource: https://example.com/.well-known/oauth-protected-resource
/// - OpenID Connect: https://example.com/.well-known/openid-configuration

use reqwest::Client;
use serde::{Deserialize, Serialize};
use turbomcp_protocol::{Error as McpError, ErrorKind, Result as McpResult};
use url::Url;

/// OAuth Authorization Server Metadata (RFC 8414)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthServerMetadata {
    /// The authorization server's issuer identifier
    pub issuer: String,

    /// URL of the authorization endpoint
    pub authorization_endpoint: String,

    /// URL of the token endpoint
    pub token_endpoint: String,

    /// URL of the token revocation endpoint (optional)
    pub revocation_endpoint: Option<String>,

    /// URL of the token introspection endpoint (optional)
    pub introspection_endpoint: Option<String>,

    /// Response types supported
    #[serde(default)]
    pub response_types_supported: Vec<String>,

    /// Grant types supported
    #[serde(default)]
    pub grant_types_supported: Vec<String>,

    /// PKCE code challenge methods supported
    #[serde(default)]
    pub code_challenge_methods_supported: Vec<String>,

    /// Scopes supported
    #[serde(default)]
    pub scopes_supported: Vec<String>,

    /// Token endpoint auth methods supported
    #[serde(default)]
    pub token_endpoint_auth_methods_supported: Vec<String>,

    /// DPoP signing algorithms supported (RFC 9449)
    #[serde(default)]
    pub dpop_signing_alg_values_supported: Vec<String>,
}

/// OAuth Protected Resource Metadata (RFC 9728)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    /// The protected resource's identifier
    pub resource: String,

    /// Authorization servers that can issue tokens for this resource
    pub authorization_servers: Vec<String>,

    /// Scopes supported by this resource
    #[serde(default)]
    pub scopes_supported: Vec<String>,

    /// Bearer token methods supported
    #[serde(default)]
    pub bearer_methods_supported: Vec<String>,

    /// Resource documentation URI
    pub resource_documentation: Option<String>,

    /// DPoP signing algorithms supported
    #[serde(default)]
    pub dpop_signing_alg_values_supported: Vec<String>,
}

/// Combined OAuth metadata from discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthMetadata {
    /// Authorization server metadata (RFC 8414)
    pub auth_server: Option<AuthServerMetadata>,

    /// Protected resource metadata (RFC 9728)
    pub protected_resource: Option<ProtectedResourceMetadata>,

    /// Discovery method used
    pub discovery_method: String,

    /// Timestamp of discovery
    pub discovered_at: String,
}

/// OAuth metadata discovery service
pub struct MetadataDiscovery {
    /// HTTP client for discovery requests
    client: Client,
}

impl MetadataDiscovery {
    /// Create a new metadata discovery service
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Discover OAuth metadata for a given server URL
    ///
    /// Tries multiple discovery methods in order:
    /// 1. Protected Resource Metadata (RFC 9728) - MCP preferred
    /// 2. Authorization Server Metadata (RFC 8414)
    /// 3. OpenID Connect Discovery
    ///
    /// # Arguments
    /// * `server_url` - Base URL of the OAuth server or protected resource
    ///
    /// # Returns
    /// Combined metadata from successful discovery attempts
    pub async fn discover(&self, server_url: &str) -> McpResult<OAuthMetadata> {
        let base_url =
            Url::parse(server_url).map_err(|e| McpError::invalid_params(format!("Invalid URL: {}", e)))?;

        // Try RFC 9728 first (Protected Resource Metadata) - this is MCP-specific
        if let Ok(protected_resource) = self.discover_protected_resource(&base_url).await {
            // Now discover the authorization server metadata
            if let Some(auth_server_url) = protected_resource.authorization_servers.first() {
                if let Ok(auth_server_url) = Url::parse(auth_server_url) {
                    if let Ok(auth_server) = self.discover_auth_server(&auth_server_url).await {
                        return Ok(OAuthMetadata {
                            auth_server: Some(auth_server),
                            protected_resource: Some(protected_resource),
                            discovery_method: "RFC 9728 + RFC 8414".to_string(),
                            discovered_at: chrono::Utc::now().to_rfc3339(),
                        });
                    }
                }
            }

            // If auth server discovery failed, return partial metadata
            return Ok(OAuthMetadata {
                auth_server: None,
                protected_resource: Some(protected_resource),
                discovery_method: "RFC 9728 (partial)".to_string(),
                discovered_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Try RFC 8414 (Authorization Server Metadata)
        if let Ok(auth_server) = self.discover_auth_server(&base_url).await {
            return Ok(OAuthMetadata {
                auth_server: Some(auth_server),
                protected_resource: None,
                discovery_method: "RFC 8414".to_string(),
                discovered_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Try OpenID Connect Discovery as fallback
        if let Ok(auth_server) = self.discover_openid_connect(&base_url).await {
            return Ok(OAuthMetadata {
                auth_server: Some(auth_server),
                protected_resource: None,
                discovery_method: "OpenID Connect".to_string(),
                discovered_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        Err(McpError::new(
            ErrorKind::Internal,
            "OAuth metadata discovery failed: no .well-known endpoints found",
        ))
    }

    /// Discover protected resource metadata (RFC 9728)
    ///
    /// Endpoint: https://example.com/.well-known/oauth-protected-resource
    async fn discover_protected_resource(
        &self,
        base_url: &Url,
    ) -> McpResult<ProtectedResourceMetadata> {
        let discovery_url = base_url
            .join("/.well-known/oauth-protected-resource")
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Invalid discovery URL: {}", e)))?;

        tracing::debug!("Discovering protected resource metadata: {}", discovery_url);

        let response = self
            .client
            .get(discovery_url.as_str())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Discovery request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(McpError::new(
                ErrorKind::Internal,
                format!(
                    "Protected resource metadata not found: HTTP {}",
                    response.status()
                ),
            ));
        }

        response
            .json::<ProtectedResourceMetadata>()
            .await
            .map_err(|e| McpError::serialization(format!("Invalid metadata JSON: {}", e)))
    }

    /// Discover authorization server metadata (RFC 8414)
    ///
    /// Endpoint: https://example.com/.well-known/oauth-authorization-server
    async fn discover_auth_server(&self, base_url: &Url) -> McpResult<AuthServerMetadata> {
        let discovery_url = base_url
            .join("/.well-known/oauth-authorization-server")
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Invalid discovery URL: {}", e)))?;

        tracing::debug!("Discovering authorization server metadata: {}", discovery_url);

        let response = self
            .client
            .get(discovery_url.as_str())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Discovery request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(McpError::new(
                ErrorKind::Internal,
                format!(
                    "Authorization server metadata not found: HTTP {}",
                    response.status()
                ),
            ));
        }

        response
            .json::<AuthServerMetadata>()
            .await
            .map_err(|e| McpError::serialization(format!("Invalid metadata JSON: {}", e)))
    }

    /// Discover OpenID Connect configuration
    ///
    /// Endpoint: https://example.com/.well-known/openid-configuration
    async fn discover_openid_connect(&self, base_url: &Url) -> McpResult<AuthServerMetadata> {
        let discovery_url = base_url
            .join("/.well-known/openid-configuration")
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Invalid discovery URL: {}", e)))?;

        tracing::debug!("Discovering OpenID Connect configuration: {}", discovery_url);

        let response = self
            .client
            .get(discovery_url.as_str())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Discovery request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(McpError::new(
                ErrorKind::Internal,
                format!(
                    "OpenID Connect configuration not found: HTTP {}",
                    response.status()
                ),
            ));
        }

        // OpenID Connect configuration is compatible with RFC 8414
        response
            .json::<AuthServerMetadata>()
            .await
            .map_err(|e| McpError::serialization(format!("Invalid metadata JSON: {}", e)))
    }

    /// Check if server supports PKCE
    pub fn supports_pkce(metadata: &OAuthMetadata) -> bool {
        if let Some(auth_server) = &metadata.auth_server {
            return auth_server
                .code_challenge_methods_supported
                .contains(&"S256".to_string());
        }
        false
    }

    /// Check if server supports DPoP
    pub fn supports_dpop(metadata: &OAuthMetadata) -> bool {
        // Check authorization server
        if let Some(auth_server) = &metadata.auth_server {
            if !auth_server.dpop_signing_alg_values_supported.is_empty() {
                return true;
            }
        }

        // Check protected resource
        if let Some(protected_resource) = &metadata.protected_resource {
            if !protected_resource.dpop_signing_alg_values_supported.is_empty() {
                return true;
            }
        }

        false
    }
}

impl Default for MetadataDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_pkce() {
        let metadata = OAuthMetadata {
            auth_server: Some(AuthServerMetadata {
                issuer: "https://example.com".to_string(),
                authorization_endpoint: "https://example.com/authorize".to_string(),
                token_endpoint: "https://example.com/token".to_string(),
                revocation_endpoint: None,
                introspection_endpoint: None,
                response_types_supported: vec!["code".to_string()],
                grant_types_supported: vec!["authorization_code".to_string()],
                code_challenge_methods_supported: vec!["S256".to_string()],
                scopes_supported: vec![],
                token_endpoint_auth_methods_supported: vec![],
                dpop_signing_alg_values_supported: vec![],
            }),
            protected_resource: None,
            discovery_method: "test".to_string(),
            discovered_at: chrono::Utc::now().to_rfc3339(),
        };

        assert!(MetadataDiscovery::supports_pkce(&metadata));
    }

    #[test]
    fn test_supports_dpop() {
        let metadata = OAuthMetadata {
            auth_server: Some(AuthServerMetadata {
                issuer: "https://example.com".to_string(),
                authorization_endpoint: "https://example.com/authorize".to_string(),
                token_endpoint: "https://example.com/token".to_string(),
                revocation_endpoint: None,
                introspection_endpoint: None,
                response_types_supported: vec!["code".to_string()],
                grant_types_supported: vec!["authorization_code".to_string()],
                code_challenge_methods_supported: vec!["S256".to_string()],
                scopes_supported: vec![],
                token_endpoint_auth_methods_supported: vec![],
                dpop_signing_alg_values_supported: vec!["ES256".to_string()],
            }),
            protected_resource: None,
            discovery_method: "test".to_string(),
            discovered_at: chrono::Utc::now().to_rfc3339(),
        };

        assert!(MetadataDiscovery::supports_dpop(&metadata));
    }
}
