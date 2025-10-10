//! MCP Protocol Operations Module
//!
//! Provides high-level MCP protocol operations with enterprise features:
//! - Tools: List and call tools with retry logic
//! - Prompts: List and get prompts with argument support
//! - Resources: List and read resources with URI handling
//!
//! All operations include:
//! - Connection validation and status checking
//! - Automatic metrics tracking (request count, last seen)
//! - Error handling with detailed context
//! - Structured logging for observability

use crate::error::{McpResult, McpStudioError};
use crate::mcp_client::connection::ManagedConnection;
use crate::types::ConnectionStatus;
use chrono::Utc;
use dashmap::DashMap;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use turbomcp_protocol::types::PromptInput;
use uuid::Uuid;

/// MCP Protocol Operations
///
/// Provides stateless operations for all MCP protocol capabilities.
/// All methods require a connection reference from the manager.
pub struct McpOperations;

impl McpOperations {
    /// Call a tool on an MCP server with retry logic
    ///
    /// Features:
    /// - Exponential backoff retry (3 attempts, 100ms → 200ms → 400ms)
    /// - Automatic metrics tracking
    /// - Connection validation
    /// - Parameter conversion from JSON to HashMap
    pub async fn call_tool(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
        server_id: Uuid,
        tool_name: &str,
        parameters: Value,
    ) -> McpResult<Value> {
        let connection = connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

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

        // Convert parameters to HashMap if it's an object
        let params = if parameters.is_object() {
            parameters.as_object().map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<HashMap<String, Value>>()
            })
        } else {
            None
        };

        // Retry logic with exponential backoff
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 0..=max_retries {
            // Calculate backoff delay (exponential: 100ms, 200ms, 400ms)
            if attempt > 0 {
                let delay_ms = 100 * (1_u64 << (attempt - 1));
                tracing::debug!(
                    "Retrying tool call '{}' after {}ms delay (attempt {}/{})",
                    tool_name,
                    delay_ms,
                    attempt + 1,
                    max_retries + 1
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            // Attempt the tool call
            match client.call_tool(tool_name, params.clone()).await {
                Ok(result) => {
                    // Success - update metrics and return
                    *connection.request_count.lock() += 1;
                    *connection.last_seen.write() = Some(Utc::now());

                    if attempt > 0 {
                        tracing::info!(
                            "Tool call '{}' succeeded on retry attempt {} for server {}",
                            tool_name,
                            attempt + 1,
                            server_id
                        );
                    } else {
                        tracing::info!(
                            "Successfully called tool '{}' on server {}",
                            tool_name,
                            server_id
                        );
                    }

                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    *connection.error_count.lock() += 1;

                    // Check if we should retry
                    if attempt < max_retries {
                        tracing::warn!(
                            "Tool call '{}' failed (attempt {}/{}), retrying: {}",
                            tool_name,
                            attempt + 1,
                            max_retries + 1,
                            last_error.as_ref().expect("last_error is Some after error")
                        );
                    } else {
                        tracing::error!(
                            "Tool call '{}' failed after {} attempts: {}",
                            tool_name,
                            max_retries + 1,
                            last_error
                                .as_ref()
                                .expect("last_error is Some after all retries")
                        );
                    }
                }
            }
        }

        // All retries exhausted - return the last error
        Err(McpStudioError::ToolCallFailed(format!(
            "Tool call '{}' failed after {} attempts: {}",
            tool_name,
            max_retries + 1,
            last_error.expect("last_error is Some after retry loop")
        )))
    }

    /// List tools available on an MCP server
    ///
    /// Returns full tool definitions with JSON schemas for auto-form generation
    pub async fn list_tools(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
        server_id: Uuid,
    ) -> McpResult<Vec<crate::types::ToolDefinition>> {
        let connection = connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = *connection.status.read();
        if !matches!(status, ConnectionStatus::Connected) {
            return Err(McpStudioError::ConnectionFailed(format!(
                "Server {} is not connected (status: {:?})",
                server_id, status
            )));
        }

        // Use TurboMCP client for all transports (now world-class implementation)
        let client_opt = connection.client.read().clone();
        let client = client_opt.ok_or_else(|| {
            McpStudioError::ConnectionFailed("MCP client not initialized".to_string())
        })?;

        // Get tools with full schemas using our enhanced method
        let tools = client.list_tools_with_schemas().await.map_err(|e| {
            *connection.error_count.lock() += 1;
            McpStudioError::ToolCallFailed(format!("Failed to list tools: {}", e))
        })?;

        // Update metrics
        *connection.request_count.lock() += 1;
        *connection.last_seen.write() = Some(Utc::now());

        // Convert from TurboMCP Tool to our ToolDefinition format
        let tool_definitions: Vec<crate::types::ToolDefinition> = tools
            .into_iter()
            .map(|tool| crate::types::ToolDefinition {
                name: tool.name,
                title: tool.title,
                description: tool.description,
                input_schema: tool.input_schema,
                output_schema: None,
                available: true,
            })
            .collect();

        tracing::info!(
            "Listed {} tools from server {}",
            tool_definitions.len(),
            server_id
        );
        Ok(tool_definitions)
    }

    /// List prompts available on an MCP server
    ///
    /// Returns full Prompt objects with schemas converted to JSON
    pub async fn list_prompts(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
        server_id: Uuid,
    ) -> McpResult<Vec<serde_json::Value>> {
        let connection = connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

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

        // List prompts using enhanced TurboMCP API - returns full Prompt objects with schemas
        let prompts = client.list_prompts().await.map_err(|e| {
            *connection.error_count.lock() += 1;
            McpStudioError::ToolCallFailed(format!("Failed to list prompts: {}", e))
        })?;

        // Update metrics
        *connection.request_count.lock() += 1;
        *connection.last_seen.write() = Some(Utc::now());

        // Convert Prompt objects to JSON values for frontend
        let prompt_values: Vec<serde_json::Value> = prompts
            .into_iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                McpStudioError::ToolCallFailed(format!("Failed to serialize prompts: {}", e))
            })?;

        tracing::info!(
            "Listed {} prompts from server {}",
            prompt_values.len(),
            server_id
        );
        Ok(prompt_values)
    }

    /// Get a specific prompt from an MCP server
    ///
    /// Supports variable substitution via PromptInput arguments
    pub async fn get_prompt(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
        server_id: Uuid,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> McpResult<serde_json::Value> {
        let connection = connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

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

        // Convert arguments from JSON to PromptInput if provided
        let prompt_input = if let Some(args) = arguments {
            Some(
                serde_json::from_value::<PromptInput>(args)
                    .map_err(McpStudioError::SerializationError)?,
            )
        } else {
            None
        };

        // Get prompt using enhanced TurboMCP API with full argument support
        let prompt_result = client.get_prompt(&name, prompt_input).await.map_err(|e| {
            *connection.error_count.lock() += 1;
            McpStudioError::ToolCallFailed(format!("Failed to get prompt '{}': {}", name, e))
        })?;

        // Update metrics
        *connection.request_count.lock() += 1;
        *connection.last_seen.write() = Some(Utc::now());

        tracing::info!(
            "Successfully retrieved prompt '{}' from server {}",
            name,
            server_id
        );

        // Convert GetPromptResult to JSON value for the frontend
        serde_json::to_value(prompt_result).map_err(|e| {
            McpStudioError::ToolCallFailed(format!("Failed to serialize prompt result: {}", e))
        })
    }

    /// List resources available on an MCP server
    ///
    /// Returns resource objects with URIs and metadata
    pub async fn list_resources(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
        server_id: Uuid,
    ) -> McpResult<Vec<serde_json::Value>> {
        let connection = connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

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

        // List resources using TurboMCP client (transport-agnostic)
        let resource_uris = client.list_resources().await.map_err(|e| {
            *connection.error_count.lock() += 1;
            McpStudioError::ToolCallFailed(format!("Failed to list resources: {}", e))
        })?;

        // Update metrics
        *connection.request_count.lock() += 1;
        *connection.last_seen.write() = Some(Utc::now());

        // Convert resource URIs to resource objects for the frontend
        let resource_values: Vec<serde_json::Value> = resource_uris
            .into_iter()
            .map(|uri| {
                serde_json::json!({
                    "uri": uri,
                    "name": uri.split("://").nth(1).unwrap_or(&uri).split('/').next_back().unwrap_or(&uri),
                    "description": format!("Resource at {}", uri),
                    "mimeType": "text/plain"
                })
            })
            .collect();

        tracing::info!(
            "Listed {} resources from server {}",
            resource_values.len(),
            server_id
        );
        Ok(resource_values)
    }

    /// Read a specific resource from an MCP server
    ///
    /// Returns resource contents as JSON value
    pub async fn read_resource(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
        server_id: Uuid,
        uri: String,
    ) -> McpResult<serde_json::Value> {
        let connection = connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

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

        // Read the resource using TurboMCP client (transport-agnostic)
        let resource_result = client.read_resource(&uri).await.map_err(|e| {
            *connection.error_count.lock() += 1;
            McpStudioError::ToolCallFailed(format!("Failed to read resource '{}': {}", uri, e))
        })?;

        // Update metrics
        *connection.request_count.lock() += 1;
        *connection.last_seen.write() = Some(Utc::now());

        tracing::info!(
            "Successfully read resource '{}' from server {}",
            uri,
            server_id
        );
        Ok(resource_result)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_operations_module_exists() {
        // Smoke test - module compiles
    }

    // TODO(testing): Add integration tests with mock MCP servers
    // - Test retry logic for call_tool
    // - Test connection validation
    // - Test metrics tracking
    // - Test error handling
}
