//! Client Initialization Module
//!
//! Handles MCP client initialization and bidirectional handler registration:
//! - Sampling handler registration (server → client LLM requests)
//! - Elicitation handler registration (server → client user input requests)
//! - Capabilities mapping and storage
//! - MCP protocol handshake
//!
//! All methods are transport-agnostic and work with any TurboMCP transport.

use crate::error::{McpResult, McpStudioError};
use crate::mcp_client::connection::ManagedConnection;
use crate::mcp_client::elicitation::{ContextAwareElicitationHandler, StudioElicitationHandler};
use crate::mcp_client::sampling::{ContextAwareSamplingHandler, StudioSamplingHandler};
use crate::mcp_client::ServerContext;
use std::sync::Arc;
use turbomcp_client::Client;
use turbomcp_transport::Transport;

use super::configuration::Configuration;

/// Client Initialization Operations
///
/// Provides stateless operations for initializing MCP clients with
/// bidirectional handler registration.
pub struct Initialization;

impl Initialization {
    /// Register bidirectional handlers (sampling + elicitation)
    ///
    /// Handlers are registered with server context for proper attribution in the UI.
    /// This enables:
    /// - Server-initiated LLM requests (sampling)
    /// - Server-initiated user input requests (elicitation)
    pub fn register_bidirectional_handlers<T: Transport>(
        client: &Client<T>,
        connection: &Arc<ManagedConnection>,
        transport_name: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) {
        // Create server context for attribution
        let server_context = ServerContext {
            server_id: connection.config.id,
            server_name: connection.config.name.clone(),
            server_description: connection.config.description.clone(),
            connected_at: chrono::Utc::now(),
        };

        // Register sampling handler (server → client LLM requests)
        tracing::info!(
            "🚀 Registering MCP Studio sampling handler for {} server-initiated requests",
            transport_name
        );
        let context_handler = Arc::new(ContextAwareSamplingHandler::new(
            server_context.clone(),
            sampling_handler,
        ));
        client.set_sampling_handler(context_handler);
        tracing::info!(
            "✅ Sampling handler registered successfully - {} server can now send sampling requests",
            transport_name
        );

        // Register elicitation handler (server → client user input requests)
        let elicitation_context_handler = Arc::new(ContextAwareElicitationHandler::new(
            server_context,
            elicitation_handler,
        ));
        client.set_elicitation_handler(elicitation_context_handler);
        tracing::info!(
            "✅ Elicitation handler registered successfully - {} server can now send elicitation requests",
            transport_name
        );
    }

    /// Register capabilities and handlers after initialization
    ///
    /// Used when you need custom error handling during initialize (e.g., ChildProcess).
    /// Takes the init_result and performs:
    /// 1. Map and store server capabilities
    /// 2. Register bidirectional handlers (sampling + elicitation)
    /// 3. Emit CapabilitiesUpdated event to frontend
    pub fn register_capabilities_and_handlers<T: Transport>(
        client: &Client<T>,
        init_result: &turbomcp_client::InitializeResult,
        connection: &Arc<ManagedConnection>,
        transport_name: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) {
        use crate::mcp_client::events::ConnectionEvent;
        use tokio::sync::mpsc::error::TrySendError;

        // Map and store server capabilities
        let server_capabilities =
            Configuration::map_server_capabilities(&init_result.server_capabilities);
        *connection.capabilities.write() = Some(server_capabilities.clone());

        // Emit CapabilitiesUpdated event to frontend for reactive UI updates
        let server_id = connection.server_id;
        match connection
            .event_sender
            .try_send(ConnectionEvent::CapabilitiesUpdated {
                server_id,
                capabilities: server_capabilities,
            }) {
            Ok(_) => {
                tracing::info!(
                    "✅ Emitted CapabilitiesUpdated event for server {}",
                    server_id
                );
            }
            Err(TrySendError::Full(_)) => {
                tracing::warn!("Event channel full, dropping CapabilitiesUpdated event");
            }
            Err(TrySendError::Closed(_)) => {
                tracing::error!("Event channel closed, cannot emit CapabilitiesUpdated event");
            }
        }

        // Register bidirectional handlers
        Self::register_bidirectional_handlers(
            client,
            connection,
            transport_name,
            sampling_handler,
            elicitation_handler,
        );
    }

    /// Finalize client initialization with MCP handshake and handler registration
    ///
    /// This is the common finalization step for most transports:
    /// 1. Perform MCP initialize handshake
    /// 2. Map and store server capabilities
    /// 3. Register bidirectional handlers (sampling + elicitation)
    /// 4. Emit CapabilitiesUpdated event to frontend
    /// 5. Log successful initialization
    ///
    /// Returns the fully initialized client ready for use.
    ///
    /// Note: For transports needing custom error handling (like ChildProcess),
    /// use `register_capabilities_and_handlers` after manual initialize instead.
    pub async fn finalize_client_initialization<T: Transport>(
        client: Client<T>,
        connection: &Arc<ManagedConnection>,
        transport_name: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<Client<T>> {
        // Perform MCP initialize handshake
        tracing::info!(
            "🔄 Performing MCP initialization handshake for {} transport...",
            transport_name
        );
        let init_result = client.initialize().await.map_err(|e| {
            McpStudioError::ConnectionFailed(format!(
                "{} MCP initialization failed: {}",
                transport_name, e
            ))
        })?;

        // Register capabilities and handlers (emits CapabilitiesUpdated event)
        Self::register_capabilities_and_handlers(
            &client,
            &init_result,
            connection,
            transport_name,
            sampling_handler,
            elicitation_handler,
        );

        // Log successful initialization with server info
        tracing::info!(
            "{} MCP client initialized successfully for server '{}' ({})",
            transport_name,
            connection.config.name,
            init_result.server_info.name
        );

        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_initialization_module_exists() {
        // Smoke test - module compiles
    }

    // TODO(testing): Add integration tests with mock transports
    // - Test handler registration
    // - Test capabilities mapping
    // - Test initialization handshake
    // - Test error handling during init
    // - Test server context creation
}
