//! Client Initialization Module
//!
//! Handles MCP client initialization and handler registration:
//! - Sampling handler registration (server → client LLM requests)
//! - Elicitation handler registration (server → client user input requests)
//! - Log handler registration (server → client log messages)
//! - Resource update handler registration (server → client resource changes)
//! - Capabilities mapping and storage
//! - MCP protocol handshake
//!
//! All methods are transport-agnostic and work with any TurboMCP transport.

use crate::error::{McpResult, McpStudioError};
use crate::mcp_client::connection::ManagedConnection;
use crate::mcp_client::elicitation::{ContextAwareElicitationHandler, StudioElicitationHandler};
use crate::mcp_client::notification_handlers::{
    ContextAwareLogHandler, ContextAwareProgressHandler, ContextAwarePromptListChangedHandler,
    ContextAwareResourceListChangedHandler, ContextAwareResourceUpdateHandler,
    ContextAwareToolListChangedHandler, StudioLogHandler, StudioProgressHandler,
    StudioPromptListChangedHandler, StudioResourceListChangedHandler, StudioResourceUpdateHandler,
    StudioToolListChangedHandler,
};
use crate::mcp_client::sampling::{ContextAwareSamplingHandler, StudioSamplingHandler};
use crate::mcp_client::ServerContext;
use std::sync::Arc;
use turbomcp_client::Client;
use turbomcp_transport::Transport;

use super::configuration::Configuration;

/// Client Initialization Operations
///
/// Provides stateless operations for initializing MCP clients with
/// all handler types registered.
pub struct Initialization;

impl Initialization {
    /// Register all MCP handlers (sampling, elicitation, log, resource update)
    ///
    /// Handlers are registered with server context for proper attribution in the UI.
    /// This enables:
    /// - Server-initiated LLM requests (sampling)
    /// - Server-initiated user input requests (elicitation)
    /// - Server log message forwarding (logging)
    /// - Resource change notifications (resource updates)
    #[allow(clippy::too_many_arguments)]
    pub fn register_all_handlers<T: Transport>(
        client: &Client<T>,
        connection: &Arc<ManagedConnection>,
        transport_name: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
        log_handler: Arc<StudioLogHandler>,
        resource_update_handler: Arc<StudioResourceUpdateHandler>,
        progress_handler: Arc<StudioProgressHandler>,
        tool_list_changed_handler: Arc<StudioToolListChangedHandler>,
        prompt_list_changed_handler: Arc<StudioPromptListChangedHandler>,
        resource_list_changed_handler: Arc<StudioResourceListChangedHandler>,
    ) {
        // Create server context for attribution
        let server_context = ServerContext {
            server_id: connection.config.id,
            server_name: connection.config.name.clone(),
            server_description: connection.config.description.clone(),
            connected_at: chrono::Utc::now(),
        };

        // Register sampling handler (server → client LLM requests)
        let sampling_ctx = Arc::new(ContextAwareSamplingHandler::new(
            server_context.clone(),
            sampling_handler,
        ));
        client.set_sampling_handler(sampling_ctx);

        // Register elicitation handler (server → client user input requests)
        let elicitation_ctx = Arc::new(ContextAwareElicitationHandler::new(
            server_context.clone(),
            elicitation_handler,
        ));
        client.set_elicitation_handler(elicitation_ctx);

        // Register log handler (server → client log messages)
        let log_ctx = Arc::new(ContextAwareLogHandler::new(
            server_context.clone(),
            log_handler,
        ));
        client.set_log_handler(log_ctx);

        // Register resource update handler (server → client resource changes)
        let resource_ctx = Arc::new(ContextAwareResourceUpdateHandler::new(
            server_context.clone(),
            resource_update_handler,
        ));
        client.set_resource_update_handler(resource_ctx);

        // Register progress handler (server → client progress notifications)
        let progress_ctx = Arc::new(ContextAwareProgressHandler::new(
            server_context.clone(),
            progress_handler,
        ));
        client.set_progress_handler(progress_ctx);

        // Register list changed handlers (server → client list change notifications)
        let tool_list_ctx = Arc::new(ContextAwareToolListChangedHandler::new(
            server_context.clone(),
            tool_list_changed_handler,
        ));
        client.set_tool_list_changed_handler(tool_list_ctx);

        let prompt_list_ctx = Arc::new(ContextAwarePromptListChangedHandler::new(
            server_context.clone(),
            prompt_list_changed_handler,
        ));
        client.set_prompt_list_changed_handler(prompt_list_ctx);

        let resource_list_ctx = Arc::new(ContextAwareResourceListChangedHandler::new(
            server_context,
            resource_list_changed_handler,
        ));
        client.set_resource_list_changed_handler(resource_list_ctx);

        tracing::info!(
            "All 8 MCP handlers registered for {} transport (sampling, elicitation, log, resource_update, progress, tool_list_changed, prompt_list_changed, resource_list_changed)",
            transport_name
        );
    }

    /// Register capabilities and handlers after initialization
    ///
    /// Used when you need custom error handling during initialize (e.g., ChildProcess).
    /// Takes the init_result and performs:
    /// 1. Map and store server capabilities
    /// 2. Register all handlers (sampling, elicitation, log, resource update)
    /// 3. Emit CapabilitiesUpdated event to frontend
    #[allow(clippy::too_many_arguments)]
    pub fn register_capabilities_and_handlers<T: Transport>(
        client: &Client<T>,
        init_result: &turbomcp_client::InitializeResult,
        connection: &Arc<ManagedConnection>,
        transport_name: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
        log_handler: Arc<StudioLogHandler>,
        resource_update_handler: Arc<StudioResourceUpdateHandler>,
        progress_handler: Arc<StudioProgressHandler>,
        tool_list_changed_handler: Arc<StudioToolListChangedHandler>,
        prompt_list_changed_handler: Arc<StudioPromptListChangedHandler>,
        resource_list_changed_handler: Arc<StudioResourceListChangedHandler>,
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
                tracing::info!("Emitted CapabilitiesUpdated event for server {}", server_id);
            }
            Err(TrySendError::Full(_)) => {
                tracing::warn!("Event channel full, dropping CapabilitiesUpdated event");
            }
            Err(TrySendError::Closed(_)) => {
                tracing::error!("Event channel closed, cannot emit CapabilitiesUpdated event");
            }
        }

        // Register all handlers
        Self::register_all_handlers(
            client,
            connection,
            transport_name,
            sampling_handler,
            elicitation_handler,
            log_handler,
            resource_update_handler,
            progress_handler,
            tool_list_changed_handler,
            prompt_list_changed_handler,
            resource_list_changed_handler,
        );
    }

    /// Finalize client initialization with MCP handshake and handler registration
    ///
    /// This is the common finalization step for most transports:
    /// 1. Perform MCP initialize handshake
    /// 2. Map and store server capabilities
    /// 3. Register all handlers (sampling, elicitation, log, resource update)
    /// 4. Emit CapabilitiesUpdated event to frontend
    /// 5. Log successful initialization
    ///
    /// Returns the fully initialized client ready for use.
    ///
    /// Note: For transports needing custom error handling (like ChildProcess),
    /// use `register_capabilities_and_handlers` after manual initialize instead.
    #[allow(clippy::too_many_arguments)]
    pub async fn finalize_client_initialization<T: Transport>(
        client: Client<T>,
        connection: &Arc<ManagedConnection>,
        transport_name: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
        log_handler: Arc<StudioLogHandler>,
        resource_update_handler: Arc<StudioResourceUpdateHandler>,
        progress_handler: Arc<StudioProgressHandler>,
        tool_list_changed_handler: Arc<StudioToolListChangedHandler>,
        prompt_list_changed_handler: Arc<StudioPromptListChangedHandler>,
        resource_list_changed_handler: Arc<StudioResourceListChangedHandler>,
    ) -> McpResult<Client<T>> {
        // Perform MCP initialize handshake
        tracing::info!(
            "Performing MCP initialization handshake for {} transport...",
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
            log_handler,
            resource_update_handler,
            progress_handler,
            tool_list_changed_handler,
            prompt_list_changed_handler,
            resource_list_changed_handler,
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
}
