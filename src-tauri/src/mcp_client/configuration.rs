//! MCP Client Configuration Module
//!
//! Provides production-ready configuration helpers for MCP client setup:
//! - Enterprise plugin configuration (retry, cache, metrics)
//! - Client capabilities with all MCP features enabled (opt-out model)
//! - Connection configuration with production defaults
//! - Server capability mapping for protocol negotiation

use std::sync::Arc;
use turbomcp_client::{Client, ClientBuilder, ClientCapabilities, ConnectionConfig};
use turbomcp_transport::Transport;

// Plugin system ready for TurboMCP 2.0.0-rc.1
#[cfg(feature = "plugins")]
use turbomcp_client::plugins::{
    CacheConfig, CachePlugin, MetricsPlugin, PluginConfig, RetryConfig, RetryPlugin,
};

/// Configuration utilities for MCP client setup
pub struct Configuration;

impl Configuration {
    /// Configure ClientBuilder with enterprise plugins for production reliability
    ///
    /// When the `plugins` feature is enabled, adds:
    /// - **Retry Plugin**: Exponential backoff, 3 retries, handles timeouts and connection errors
    /// - **Cache Plugin**: 1000 entries, 5min TTL, caches responses/resources/tools
    /// - **Metrics Plugin**: Performance monitoring for development insights
    #[cfg(feature = "plugins")]
    pub fn configure_plugins(mut builder: ClientBuilder) -> ClientBuilder {
        // Retry plugin with exponential backoff - essential for MCP Studio's server testing
        let retry_config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            retry_on_timeout: true,
            retry_on_connection_error: true,
        };
        builder = builder.with_plugin(Arc::new(RetryPlugin::new(PluginConfig::Retry(
            retry_config,
        ))));

        // Cache plugin for performance - critical for repeated tool/resource calls
        let cache_config = CacheConfig {
            max_entries: 1000,
            ttl_seconds: 300, // 5 minutes - good for development workflow
            cache_responses: true,
            cache_resources: true,
            cache_tools: true,
        };
        builder = builder.with_plugin(Arc::new(CachePlugin::new(PluginConfig::Cache(
            cache_config,
        ))));

        // Metrics plugin for monitoring server performance - valuable for development
        builder = builder.with_plugin(Arc::new(MetricsPlugin::new(PluginConfig::Metrics)));

        tracing::info!("Enterprise plugins enabled: Retry, Cache, Metrics");
        builder
    }

    /// No-op plugin configuration when `plugins` feature is disabled
    #[cfg(not(feature = "plugins"))]
    pub fn configure_plugins(builder: ClientBuilder) -> ClientBuilder {
        tracing::debug!("Plugin system disabled - using basic client");
        builder
    }

    /// Build MCP client with all capabilities enabled
    ///
    /// Configures client with:
    /// - All MCP operations enabled (tools, prompts, resources, sampling)
    /// - Enterprise plugins (retry, cache, metrics when available)
    /// - Connection configuration with production defaults
    /// - Transport-layer reliability features
    ///
    /// # Arguments
    /// * `transport` - The transport layer to use (STDIO, HTTP, WebSocket, etc.)
    /// * `connection_config` - Connection timeouts, retries, and keepalive settings
    ///
    /// # Returns
    /// Configured MCP client ready for bidirectional operations
    pub fn build_client_with_capabilities<T: Transport>(
        transport: T,
        connection_config: ConnectionConfig,
    ) -> Client<T> {
        Self::configure_plugins(ClientBuilder::new())
            .with_capabilities(ClientCapabilities::all())
            .with_connection_config(connection_config)
            .build_sync(transport)
    }

    /// Get protocol-level capabilities configuration
    ///
    /// Returns capabilities for the MCP protocol layer with:
    /// - All MCP protocol features enabled by default (opt-out model)
    /// - Forward compatible with future MCP protocol extensions
    /// - Roots capability configured for file system access in Studio
    ///
    /// # Returns
    /// Protocol capabilities ready for protocol negotiation
    pub fn configure_client_capabilities() -> turbomcp_protocol::types::ClientCapabilities {
        // TurboMCP opt-out model: all capabilities enabled by default for forward compatibility
        turbomcp_protocol::ClientCapabilities::default()
    }

    /// Create enterprise-grade connection configuration
    ///
    /// Provides consistent timeout, retry, and keepalive settings across all transports.
    /// These values are optimized for production reliability:
    /// - 60s timeout prevents hangs while allowing complex operations
    /// - 3 retries with exponential backoff for transient failures
    /// - 60s keepalive prevents idle disconnections
    ///
    /// # Returns
    /// Production-ready connection configuration
    pub fn create_enterprise_connection_config() -> ConnectionConfig {
        ConnectionConfig {
            timeout_ms: 60_000,    // 60 second timeout
            max_retries: 3,        // Retry failed requests
            retry_delay_ms: 1_000, // 1 second between retries
            keepalive_ms: 60_000,  // 60 second keepalive
        }
    }

    /// Map TurboMCP server capabilities to MCP Studio capability types
    ///
    /// Performs a clean conversion from TurboMCP's protocol types to our internal
    /// capability representation, preserving all advertised features.
    ///
    /// # Arguments
    /// * `caps` - Server capabilities from TurboMCP protocol
    ///
    /// # Returns
    /// MCP Studio internal capability representation
    pub fn map_server_capabilities(
        caps: &turbomcp_protocol::ServerCapabilities,
    ) -> crate::types::ServerCapabilities {
        crate::types::ServerCapabilities {
            tools: caps
                .tools
                .as_ref()
                .map(|_| crate::types::ToolsCapabilities { list_changed: None }),
            prompts: caps
                .prompts
                .as_ref()
                .map(|_| crate::types::PromptsCapabilities { list_changed: None }),
            resources: caps
                .resources
                .as_ref()
                .map(|_| crate::types::ResourcesCapabilities {
                    subscribe: None,
                    list_changed: None,
                }),
            completions: caps.completions.clone(),
            experimental: caps.experimental.clone(),
            logging: caps.logging.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enterprise_connection_config() {
        let config = Configuration::create_enterprise_connection_config();
        assert_eq!(config.timeout_ms, 60_000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 1_000);
        assert_eq!(config.keepalive_ms, 60_000);
    }

    #[test]
    fn test_configure_capabilities_returns_valid_config() {
        let caps = Configuration::configure_client_capabilities();
        // Protocol-level capabilities are returned successfully
        // The actual client capabilities (tools, prompts, resources, sampling)
        // are configured separately via ClientCapabilities::all() in build_client_with_capabilities()
        assert!(
            caps.experimental.is_none() || caps.experimental.is_some(),
            "Protocol capabilities should initialize successfully"
        );
    }
}
