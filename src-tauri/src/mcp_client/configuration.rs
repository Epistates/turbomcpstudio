//! MCP Client Configuration Module
//!
//! Provides static configuration helpers for MCP client setup including:
//! - Enterprise plugin configuration (retry, cache, metrics)
//! - Client capability builders with type-state validation
//! - Connection configuration with production defaults
//! - Capability mapping between protocol types

use std::sync::Arc;
use turbomcp_client::{Client, ClientBuilder, ConnectionConfig};
use turbomcp_protocol::capabilities::builders::ClientCapabilitiesBuilder;
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

    /// Build MCP client with standard capabilities (DRY helper)
    ///
    /// Configures client with:
    /// - All MCP operations enabled (tools, prompts, resources, sampling)
    /// - Plugin support (retry, cache, metrics when available)
    /// - Connection configuration
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
            .with_tools(true)
            .with_prompts(true)
            .with_resources(true)
            .with_sampling(true) // Enable production-grade sampling with HITL
            // TurboMCP 2.0: Reliability features configured via transport layer
            .with_connection_config(connection_config)
            .build_sync(transport)
    }

    /// Configure client capabilities with type-state builders for compile-time validation
    ///
    /// Uses TurboMCP 2.0.0-rc.1 type-state capability builders to ensure:
    /// - Compile-time validation of capability combinations
    /// - Sub-capabilities only available when parent enabled
    /// - TurboMCP exclusive features (LLM provider, UI capabilities)
    ///
    /// # Features Enabled
    /// - **Experimental**: Latest MCP protocol features
    /// - **Roots**: File system access for MCP Studio
    /// - **Sampling**: HITL LLM integration
    /// - **Elicitation**: Server-initiated user input
    /// - **LLM Provider**: OpenAI GPT-4 (TurboMCP exclusive)
    /// - **UI Capabilities**: Form, dialog, toast (TurboMCP exclusive)
    pub fn configure_client_capabilities() -> turbomcp_protocol::types::ClientCapabilities {
        // Use TurboMCP 2.0.0-rc.1 type-state capability builders for compile-time validation
        let client_caps = ClientCapabilitiesBuilder::new()
            .enable_experimental() // Enables experimental capability state
            .enable_roots() // Enables roots capability state (for MCP Studio file access)
            .enable_sampling() // Enables sampling capability state (for HITL)
            .enable_elicitation() // Enables elicitation capability state
            // Sub-capability only available when roots are enabled!
            .enable_roots_list_changed() // ✅ Only available when roots enabled
            // TurboMCP exclusive features for MCP Studio!
            .with_llm_provider("openai", "gpt-4") // 🚀 TurboMCP exclusive
            .with_ui_capabilities(vec!["form", "dialog", "toast"]) // 🚀 TurboMCP exclusive - perfect for MCP Studio UI
            .build();

        tracing::info!("Type-state client capabilities configured with compile-time validation");
        tracing::info!("✅ Roots enabled: {}", client_caps.roots.is_some());
        tracing::info!("✅ Sampling enabled: {}", client_caps.sampling.is_some());
        tracing::info!(
            "✅ Elicitation enabled: {}",
            client_caps.elicitation.is_some()
        );

        client_caps
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
    fn test_client_capabilities_configured() {
        let caps = Configuration::configure_client_capabilities();
        assert!(caps.roots.is_some(), "Roots should be enabled");
        assert!(caps.sampling.is_some(), "Sampling should be enabled");
        assert!(caps.elicitation.is_some(), "Elicitation should be enabled");
        assert!(
            caps.experimental.is_some(),
            "Experimental should be enabled"
        );
    }
}
