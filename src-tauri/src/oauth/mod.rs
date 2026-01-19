/// OAuth 2.1 Visual Debugger Module
///
/// Provides comprehensive OAuth authentication support for TurboMCP Studio,
/// including:
/// - Authorization Code Flow with PKCE (RFC 7636)
/// - RFC 8707 (Resource Indicators) - MCP required
/// - RFC 9728 (Protected Resource Metadata) - MCP required
/// - RFC 7591 (Dynamic Client Registration)
/// - RFC 9449 (DPoP) - optional enhancement
/// - Multi-provider support (Google, GitHub, Microsoft, Custom)
///
/// Architecture:
/// - `flow_manager`: OAuth flow state machine and orchestration
/// - `callback_server`: Local HTTP server for OAuth redirects (localhost:8080)
/// - `metadata_discovery`: RFC 8414/9728 auto-configuration
/// - `token_store`: Secure token persistence with system keyring
/// - `dpop_manager`: DPoP proof generation (optional)
///
/// Note: This module is WIP - some helper functions are implemented but not yet
/// integrated with the main OAuth flow.

#[allow(dead_code)]
pub mod callback_server;
#[allow(dead_code)]
pub mod dpop_manager;
pub mod flow_manager;
#[allow(dead_code)]
pub mod metadata_discovery;
pub mod provider_templates;
#[allow(dead_code)]
pub mod token_store;

// Re-export key types for convenience
pub use flow_manager::{OAuthConfig, OAuthFlowManager};

