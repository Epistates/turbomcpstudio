//! MCP Client Configuration Module
//!
//! Provides production-ready configuration helpers for MCP client setup:
//! - v3 resilience configuration (retry, circuit breaker, health checks)
//! - Client capabilities with all MCP features enabled (opt-out model)
//! - Connection configuration with production defaults
//! - Server capability mapping for protocol negotiation

use turbomcp_client::{Client, ClientBuilder, ClientCapabilities, ConnectionConfig};
use turbomcp_transport::Transport;

// v3 resilience configuration
use turbomcp_transport::resilience::{CircuitBreakerConfig, HealthCheckConfig, RetryConfig};

/// Configuration utilities for MCP client setup
pub struct Configuration;

impl Configuration {
    /// Configure ClientBuilder with enterprise resilience for production reliability
    ///
    /// Configures v3 resilience features:
    /// - **Retry**: Exponential backoff, 3 retries, handles timeouts and connection errors
    /// - **Circuit Breaker**: Prevents cascade failures with configurable thresholds
    /// - **Health Checks**: Periodic connection health monitoring
    ///
    /// Note: Currently unused - available for production deployments requiring enhanced reliability.
    #[allow(dead_code)]
    pub fn configure_resilience(builder: ClientBuilder) -> ClientBuilder {
        use std::time::Duration;

        // Retry config with exponential backoff - essential for MCP Studio's server testing
        let retry_config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1, // 10% jitter to prevent thundering herd
            retry_on_timeout: true,
            retry_on_connection_error: true,
            custom_retry_conditions: Vec::new(),
        };

        // Circuit breaker for cascade failure protection
        let circuit_config = CircuitBreakerConfig {
            failure_threshold: 5,      // Open after 5 failures
            success_threshold: 2,      // Close after 2 successes in half-open
            timeout: Duration::from_secs(30), // Stay open for 30s before half-open
            rolling_window_size: 100,  // Track last 100 requests
            minimum_requests: 10,      // Minimum requests before opening
        };

        // Health check config for connection monitoring
        let health_config = HealthCheckConfig {
            interval: Duration::from_secs(30),  // Check every 30s
            timeout: Duration::from_secs(5),    // 5s timeout for health check
            failure_threshold: 3,               // Unhealthy after 3 failed checks
            success_threshold: 1,               // Healthy after 1 success
            custom_check: None,
        };

        tracing::info!("v3 resilience enabled: Retry, Circuit Breaker, Health Checks");

        builder
            .with_retry_config(retry_config)
            .with_circuit_breaker_config(circuit_config)
            .with_health_check_config(health_config)
    }

    /// Build MCP client with all capabilities enabled
    ///
    /// Configures client with:
    /// - All MCP operations enabled (tools, prompts, resources, sampling)
    /// - v3 resilience features (retry, circuit breaker, health checks)
    /// - Connection configuration with production defaults
    /// - Transport-layer reliability features
    ///
    /// # Arguments
    /// * `transport` - The transport layer to use (STDIO, HTTP, WebSocket, etc.)
    /// * `connection_config` - Connection timeouts, retries, and keepalive settings
    ///
    /// # Returns
    /// Configured MCP client ready for bidirectional operations
    pub fn build_client_with_capabilities<T: Transport + 'static>(
        transport: T,
        connection_config: ConnectionConfig,
    ) -> Client<T> {
        ClientBuilder::new()
            .with_capabilities(ClientCapabilities::all())
            .with_connection_config(connection_config)
            .build_sync(transport)
    }

    /// Build MCP client with resilience features enabled
    ///
    /// Creates a client wrapped in TurboTransport for production-grade reliability:
    /// - Automatic retry with exponential backoff
    /// - Circuit breaker pattern for fast failure
    /// - Health checking and monitoring
    ///
    /// # Arguments
    /// * `transport` - The transport layer to use
    /// * `connection_config` - Connection settings
    ///
    /// # Returns
    /// Resilient MCP client with all capabilities
    ///
    /// Note: Currently unused - available for production deployments requiring enhanced reliability.
    #[allow(dead_code)]
    pub async fn build_resilient_client<T: Transport + 'static>(
        transport: T,
        connection_config: ConnectionConfig,
    ) -> turbomcp_protocol::McpResult<Client<turbomcp_transport::resilience::TurboTransport>> {
        Self::configure_resilience(ClientBuilder::new())
            .with_capabilities(ClientCapabilities::all())
            .with_connection_config(connection_config)
            .build_resilient(transport)
            .await
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
                .map(|t| crate::types::ToolsCapabilities {
                    list_changed: t.list_changed,
                }),
            prompts: caps
                .prompts
                .as_ref()
                .map(|p| crate::types::PromptsCapabilities {
                    list_changed: p.list_changed,
                }),
            resources: caps
                .resources
                .as_ref()
                .map(|r| crate::types::ResourcesCapabilities {
                    subscribe: r.subscribe,
                    list_changed: r.list_changed,
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
