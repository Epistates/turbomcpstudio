//! Miscellaneous Operations Module
//!
//! Provides utility operations for MCP connections:
//! - Completion suggestions (autocompletion for prompts/resources)
//! - Handler status checking (verify event handler registration)
//! - Filesystem roots listing (deprecated in MCP 2025-06-18)

use crate::error::{McpResult, McpStudioError};
use crate::mcp_client::connection::ManagedConnection;
use crate::types::ConnectionStatus;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Miscellaneous Operations
///
/// Provides stateless operations for utility functions that don't fit other modules.
pub struct MiscOperations;

impl MiscOperations {
    /// Get autocompletion suggestions for prompt arguments and resource URIs
    ///
    /// Uses TurboMCP's completion API to provide intelligent suggestions.
    pub async fn get_completions(
        connection: &Arc<ManagedConnection>,
        server_id: Uuid,
        completion_name: String,
        partial_input: String,
    ) -> McpResult<Vec<String>> {
        // Check connection status
        let status = *connection.status.read();
        if !matches!(status, ConnectionStatus::Connected) {
            return Err(McpStudioError::ConnectionFailed(format!(
                "Server {} is not connected (status: {:?})",
                server_id, status
            )));
        }

        // Get the TurboMCP client
        let client_opt = connection.client.read().clone();
        let client = client_opt.ok_or_else(|| {
            McpStudioError::ConnectionFailed("MCP client not initialized".to_string())
        })?;

        // Request completions using TurboMCP client
        let completions = client
            .complete(&completion_name, &partial_input)
            .await
            .map_err(|e| {
                *connection.error_count.lock() += 1;
                McpStudioError::ToolCallFailed(format!("Failed to get completions: {}", e))
            })?;

        // Update metrics
        *connection.request_count.lock() += 1;
        *connection.last_seen.write() = Some(Utc::now());

        tracing::info!(
            "Retrieved {} completions for '{}' from server {}",
            completions.len(),
            completion_name,
            server_id
        );

        Ok(completions)
    }

    /// List filesystem roots (deprecated in MCP 2025-06-18)
    ///
    /// Per MCP 2025-06-18 specification, roots/list is SERVER→CLIENT (not CLIENT→SERVER).
    /// Servers request roots from clients, not vice versa.
    /// This method returns an empty list with a deprecation warning.
    pub async fn list_filesystem_roots(
        connection: &Arc<ManagedConnection>,
        server_id: Uuid,
    ) -> McpResult<Vec<String>> {
        // Check connection status
        let status = *connection.status.read();
        if !matches!(status, ConnectionStatus::Connected) {
            return Err(McpStudioError::ConnectionFailed(format!(
                "Server {} is not connected (status: {:?})",
                server_id, status
            )));
        }

        // TurboMCP 2.0 Breaking Change: list_roots() removed
        // Per MCP 2025-06-18 spec, roots/list is SERVER→CLIENT (not CLIENT→SERVER)
        // Servers request roots from clients, not vice versa
        // Clients should implement roots handler to respond to server requests

        tracing::warn!(
            "list_filesystem_roots() called but roots/list is SERVER→CLIENT in MCP 2025-06-18. \
             Server {} should implement roots handler instead.",
            server_id
        );

        // Return empty list with deprecation notice
        Ok(vec![])
    }

    /// Check handler registration status for a server (TurboMCP 1.0.10)
    ///
    /// Returns which event handlers are currently registered:
    /// - Elicitation handler (user input requests)
    /// - Progress handler (operation progress updates)
    /// - Log handler (server logging)
    /// - Resource update handler (resource change notifications)
    pub async fn get_handler_status(
        connection: &Arc<ManagedConnection>,
        server_id: Uuid,
    ) -> McpResult<serde_json::Value> {
        // Check connection status
        let status = *connection.status.read();
        if !matches!(status, ConnectionStatus::Connected) {
            return Err(McpStudioError::ConnectionFailed(format!(
                "Server {} is not connected (status: {:?})",
                server_id, status
            )));
        }

        // Get the TurboMCP client
        let client_opt = connection.client.read().clone();
        let client = client_opt.ok_or_else(|| {
            McpStudioError::ConnectionFailed("MCP client not initialized".to_string())
        })?;

        // Check all handler registration statuses
        let elicitation_handler = client.has_elicitation_handler().await;
        let progress_handler = client.has_progress_handler().await;
        let log_handler = client.has_log_handler().await;
        let resource_update_handler = client.has_resource_update_handler().await;

        // Update metrics
        *connection.request_count.lock() += 1;
        *connection.last_seen.write() = Some(Utc::now());

        // Count registered handlers
        let handlers = [
            elicitation_handler,
            progress_handler,
            log_handler,
            resource_update_handler,
        ];
        let total_registered = handlers.iter().filter(|&&x| x).count();

        let status_response = serde_json::json!({
            "server_id": server_id,
            "handlers": {
                "elicitation": elicitation_handler,
                "progress": progress_handler,
                "log": log_handler,
                "resource_update": resource_update_handler
            },
            "total_registered": total_registered
        });

        tracing::info!(
            "Retrieved handler status for server {}: {} handlers registered",
            server_id,
            total_registered
        );

        Ok(status_response)
    }
}
