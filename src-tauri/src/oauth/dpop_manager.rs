//! DPoP (Demonstrating Proof-of-Possession) - PLACEHOLDER IMPLEMENTATION
//!
//! WARNING: This module does NOT implement DPoP (RFC 9449). The `generate_key_pair`
//! and `generate_proof` functions return errors to prevent callers from believing
//! DPoP binding is active when it is not. Do not enable `use_dpop` in OAuth
//! configurations until a real implementation is provided.
//!
//! When implemented, this module will provide:
//! - Cryptographic proof that the client presenting a token is the same client
//!   that the token was issued to
//! - Protection against token theft and replay attacks
//! - ES256 (ECDSA with P-256 and SHA-256) signature algorithm

#[cfg(feature = "dpop")]
use parking_lot::RwLock;
#[cfg(feature = "dpop")]
use std::collections::HashMap;
#[cfg(feature = "dpop")]
use std::sync::Arc;
#[cfg(feature = "dpop")]
use turbomcp_protocol::{Error as McpError, ErrorKind, Result as McpResult};

/// DPoP key pair for a server
#[cfg(feature = "dpop")]
#[derive(Debug, Clone)]
pub struct DPoPKeyPair {
    /// JWK thumbprint (jkt)
    pub thumbprint: String,

    /// Public key (for verification)
    pub public_key: String,

    /// Private key (for signing)
    /// In a real implementation, this would be stored more securely
    pub private_key: String,

    /// Created timestamp
    pub created_at: i64,
}

/// DPoP proof manager
#[cfg(feature = "dpop")]
pub struct DPoPManager {
    /// Key pairs by server_id
    key_pairs: Arc<RwLock<HashMap<i64, DPoPKeyPair>>>,
}

#[cfg(feature = "dpop")]
impl DPoPManager {
    /// Create a new DPoP manager
    pub fn new() -> Self {
        Self {
            key_pairs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new DPoP key pair for a server
    ///
    /// # Arguments
    /// * `server_id` - Unique identifier for the MCP server
    ///
    /// # Returns
    /// Error — DPoP key generation is not yet implemented.
    pub async fn generate_key_pair(&self, server_id: i64) -> McpResult<String> {
        tracing::error!(
            "DPoP key generation is a placeholder — DPoP binding is NOT active (server {})",
            server_id
        );
        Err(McpError::new(
            ErrorKind::Internal,
            "DPoP is not yet implemented. Disable use_dpop in your OAuth configuration.",
        ))
    }

    /// Generate a DPoP proof for a request
    ///
    /// # Arguments
    /// * `server_id` - Server to generate proof for
    /// * `htm` - HTTP method (e.g., "GET", "POST")
    /// * `htu` - HTTP URI (target URL)
    /// * `access_token` - Optional access token to bind proof to
    ///
    /// # Returns
    /// Error — DPoP proof generation is not yet implemented.
    pub async fn generate_proof(
        &self,
        server_id: i64,
        htm: &str,
        htu: &str,
        access_token: Option<&str>,
    ) -> McpResult<String> {
        let _ = (server_id, htm, htu, access_token); // Suppress unused warnings
        Err(McpError::new(
            ErrorKind::Internal,
            "DPoP is not yet implemented. Disable use_dpop in your OAuth configuration.",
        ))
    }

    /// Get JWK thumbprint for a server
    pub async fn get_thumbprint(&self, server_id: i64) -> McpResult<String> {
        let key_pairs = self.key_pairs.read();
        let key_pair = key_pairs.get(&server_id).ok_or_else(|| {
            McpError::new(
                ErrorKind::Internal,
                format!("No DPoP key pair found for server {}", server_id),
            )
        })?;

        Ok(key_pair.thumbprint.clone())
    }

    /// Delete key pair for server
    pub async fn delete_key_pair(&self, server_id: i64) -> McpResult<()> {
        self.key_pairs.write().remove(&server_id);
        Ok(())
    }

    /// Check if server has DPoP key pair
    pub fn has_key_pair(&self, server_id: i64) -> bool {
        self.key_pairs.read().contains_key(&server_id)
    }
}

#[cfg(feature = "dpop")]
impl Default for DPoPManager {
    fn default() -> Self {
        Self::new()
    }
}

// If dpop feature is not enabled, provide stub implementations
#[cfg(not(feature = "dpop"))]
pub struct DPoPManager;

#[cfg(not(feature = "dpop"))]
impl DPoPManager {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "dpop"))]
impl Default for DPoPManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "dpop"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_key_pair_returns_err() {
        // DPoP is a placeholder — key generation must return an error to
        // prevent callers from believing DPoP binding is active.
        let manager = DPoPManager::new();
        let result = manager.generate_key_pair(1).await;
        assert!(result.is_err(), "Expected generate_key_pair to return Err");
        assert!(!manager.has_key_pair(1));
    }

    #[tokio::test]
    async fn test_generate_proof_returns_err() {
        let manager = DPoPManager::new();
        let result = manager
            .generate_proof(1, "POST", "https://example.com/api", None)
            .await;
        assert!(result.is_err(), "Expected generate_proof to return Err");
    }

    #[tokio::test]
    async fn test_delete_key_pair() {
        let manager = DPoPManager::new();
        // generate_key_pair now returns Err, so no key is inserted
        assert!(!manager.has_key_pair(1));
        manager.delete_key_pair(1).await.unwrap();
        assert!(!manager.has_key_pair(1));
    }
}
