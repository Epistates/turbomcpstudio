//! Transport-agnostic MCP client wrapper
//!
//! This module provides a unified interface for MCP clients across different
//! transport types (STDIO, HTTP, WebSocket, TCP, Unix sockets, etc.)

use std::collections::HashMap;
use turbomcp_client::handlers::{
    ElicitationHandler, LogHandler, ProgressHandler, PromptListChangedHandler,
    ResourceListChangedHandler, ResourceUpdateHandler, ToolListChangedHandler,
};
use turbomcp_client::Client;
use turbomcp_protocol::types::{
    CallToolResult, GetPromptResult, Prompt, PromptInput, ReadResourceResult, Tool, ToolInputSchema,
};
use turbomcp_protocol::{ErrorKind, McpError}; // v3.0: Unified error type
use turbomcp_stdio::StdioTransport;
use turbomcp_transport::child_process::ChildProcessTransport;

use crate::interceptor::InterceptedTransport;

#[cfg(feature = "http")]
use turbomcp_http::StreamableHttpClientTransport;

#[cfg(feature = "websocket")]
use turbomcp_websocket::WebSocketBidirectionalTransport;

#[cfg(feature = "tcp")]
use turbomcp_tcp::TcpTransport;

#[cfg(unix)]
use turbomcp_unix::UnixTransport;

/// Transport-agnostic MCP client wrapper
///
/// Following TurboMCP's world-class Clone pattern (reqwest/AWS SDK style),
/// all clients are now cloneable directly - no SharedClient wrapper needed!
///
/// Note: Non-intercepted variants (Stdio, ChildProcess, Http, etc.) exist for
/// when protocol interception is disabled. Currently we always use Intercepted
/// variants for Protocol Inspector integration.
#[derive(Clone)]
#[allow(dead_code)] // Non-intercepted variants reserved for future use
pub enum McpTransportClient {
    Stdio(Client<StdioTransport>),
    ChildProcess(Client<ChildProcessTransport>),

    /// ChildProcess transport with protocol interception for Protocol Inspector
    InterceptedChildProcess(Client<InterceptedTransport<ChildProcessTransport>>),

    #[cfg(feature = "http")]
    Http(Client<StreamableHttpClientTransport>),

    /// HTTP transport with protocol interception for Protocol Inspector
    #[cfg(feature = "http")]
    InterceptedHttp(Client<InterceptedTransport<StreamableHttpClientTransport>>),

    #[cfg(feature = "websocket")]
    WebSocket(Client<WebSocketBidirectionalTransport>),

    /// WebSocket transport with protocol interception for Protocol Inspector
    #[cfg(feature = "websocket")]
    InterceptedWebSocket(Client<InterceptedTransport<WebSocketBidirectionalTransport>>),

    #[cfg(feature = "tcp")]
    Tcp(Client<TcpTransport>),

    /// TCP transport with protocol interception for Protocol Inspector
    #[cfg(feature = "tcp")]
    InterceptedTcp(Client<InterceptedTransport<TcpTransport>>),

    #[cfg(unix)]
    Unix(Client<UnixTransport>),

    /// Unix socket transport with protocol interception for Protocol Inspector
    #[cfg(unix)]
    InterceptedUnix(Client<InterceptedTransport<UnixTransport>>),
}

impl McpTransportClient {
    /// Helper to convert HTTP string errors to TurboMCP McpError
    #[cfg(feature = "http")]
    fn http_error(kind: ErrorKind, message: String) -> McpError {
        // v3.0: McpError::new returns McpError directly
        McpError::new(kind, message)
    }

    /// Create a basic tool schema for tools where we don't have full schema information
    #[allow(dead_code)]
    fn create_basic_tool_schema(_name: &str) -> ToolInputSchema {
        // Create an empty schema that allows any properties
        // The actual tool will define its own parameter schema
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(HashMap::new()),
            required: Some(vec![]),
            additional_properties: Some(true),
        }
    }

    /// Get tools with their schemas from the MCP server
    /// This method is now an alias for list_tools() since TurboMCP 2.0.0-rc.1
    /// returns full Tool objects with schemas by default
    pub async fn list_tools_with_schemas(&self) -> Result<Vec<Tool>, McpError> {
        tracing::info!("Getting tool schemas using TurboMCP 2.0.0-rc.1 API");
        self.list_tools().await
    }

    /// Call a tool on the MCP server (transport-agnostic)
    ///
    /// v3 Note: Returns CallToolResult serialized as serde_json::Value for backward compatibility.
    /// The result contains `content`, `is_error`, and optional `structured_content`.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        parameters: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<serde_json::Value, McpError> {
        // v3: call_tool returns CallToolResult, convert to JSON for backward compatibility
        let convert_result = |result: CallToolResult| -> Result<serde_json::Value, McpError> {
            serde_json::to_value(result).map_err(|e| {
                McpError::new(
                    ErrorKind::Internal,
                    format!("Failed to serialize CallToolResult: {}", e),
                )
            })
        };

        match self {
            McpTransportClient::Stdio(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }
            McpTransportClient::ChildProcess(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                let result: CallToolResult = client
                    .call_tool(tool_name, parameters, None)
                    .await
                    .map_err(|e| {
                        Self::http_error(
                            ErrorKind::Transport,
                            format!("HTTP call_tool failed: {}", e),
                        )
                    })?;
                convert_result(result)
            }

            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                let result: CallToolResult = client
                    .call_tool(tool_name, parameters, None)
                    .await
                    .map_err(|e| {
                        Self::http_error(
                            ErrorKind::Transport,
                            format!("HTTP call_tool failed: {}", e),
                        )
                    })?;
                convert_result(result)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }

            #[cfg(unix)]
            McpTransportClient::Unix(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }

            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                let result: CallToolResult = client.call_tool(tool_name, parameters, None).await?;
                convert_result(result)
            }
        }
    }

    /// List tools available on the MCP server (transport-agnostic)
    pub async fn list_tools(&self) -> Result<Vec<Tool>, McpError> {
        match self {
            McpTransportClient::Stdio(client) => client.list_tools().await,
            McpTransportClient::ChildProcess(client) => client.list_tools().await,
            McpTransportClient::InterceptedChildProcess(client) => client.list_tools().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.list_tools().await.map_err(|e| {
                Self::http_error(
                    ErrorKind::Transport,
                    format!("HTTP list_tools failed: {}", e),
                )
            }),

            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.list_tools().await.map_err(|e| {
                Self::http_error(
                    ErrorKind::Transport,
                    format!("HTTP list_tools failed: {}", e),
                )
            }),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.list_tools().await,

            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => client.list_tools().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.list_tools().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.list_tools().await,

            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.list_tools().await,

            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.list_tools().await,
        }
    }

    /// List prompts available on the MCP server (transport-agnostic)
    pub async fn list_prompts(&self) -> Result<Vec<Prompt>, McpError> {
        match self {
            McpTransportClient::Stdio(client) => client.list_prompts().await,
            McpTransportClient::ChildProcess(client) => client.list_prompts().await,
            McpTransportClient::InterceptedChildProcess(client) => client.list_prompts().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.list_prompts().await.map_err(|e| {
                Self::http_error(
                    ErrorKind::Transport,
                    format!("HTTP list_prompts failed: {}", e),
                )
            }),

            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                client.list_prompts().await.map_err(|e| {
                    Self::http_error(
                        ErrorKind::Transport,
                        format!("HTTP list_prompts failed: {}", e),
                    )
                })
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.list_prompts().await,

            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => client.list_prompts().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.list_prompts().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.list_prompts().await,

            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.list_prompts().await,

            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.list_prompts().await,
        }
    }

    /// Get a specific prompt from the MCP server (transport-agnostic)
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<PromptInput>,
    ) -> Result<serde_json::Value, McpError> {
        match self {
            McpTransportClient::Stdio(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }
            McpTransportClient::ChildProcess(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                let result: GetPromptResult =
                    client.get_prompt(name, arguments).await.map_err(|e| {
                        Self::http_error(
                            ErrorKind::Transport,
                            format!("HTTP get_prompt failed: {}", e),
                        )
                    })?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                let result: GetPromptResult =
                    client.get_prompt(name, arguments).await.map_err(|e| {
                        Self::http_error(
                            ErrorKind::Transport,
                            format!("HTTP get_prompt failed: {}", e),
                        )
                    })?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(unix)]
            McpTransportClient::Unix(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                let result: GetPromptResult = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }
        }
    }

    /// List resources available on the MCP server (transport-agnostic)
    /// Returns full Resource objects per MCP spec (as of TurboMCP 2.0.1)
    pub async fn list_resources(&self) -> Result<Vec<turbomcp_client::Resource>, McpError> {
        match self {
            McpTransportClient::Stdio(client) => client.list_resources().await,
            McpTransportClient::ChildProcess(client) => client.list_resources().await,
            McpTransportClient::InterceptedChildProcess(client) => client.list_resources().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.list_resources().await.map_err(|e| {
                Self::http_error(
                    ErrorKind::Transport,
                    format!("HTTP list_resources failed: {}", e),
                )
            }),

            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                client.list_resources().await.map_err(|e| {
                    Self::http_error(
                        ErrorKind::Transport,
                        format!("HTTP list_resources failed: {}", e),
                    )
                })
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.list_resources().await,

            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => client.list_resources().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.list_resources().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.list_resources().await,

            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.list_resources().await,

            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.list_resources().await,
        }
    }

    /// Read a specific resource from the MCP server (transport-agnostic)
    pub async fn read_resource(&self, uri: &str) -> Result<serde_json::Value, McpError> {
        match self {
            McpTransportClient::Stdio(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }
            McpTransportClient::ChildProcess(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await.map_err(|e| {
                    Self::http_error(
                        ErrorKind::Transport,
                        format!("HTTP read_resource failed: {}", e),
                    )
                })?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await.map_err(|e| {
                    Self::http_error(
                        ErrorKind::Transport,
                        format!("HTTP read_resource failed: {}", e),
                    )
                })?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(unix)]
            McpTransportClient::Unix(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                let result: ReadResourceResult = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }
        }
    }

    /// Get the transport type for this client
    #[allow(dead_code)]
    pub fn transport_type(&self) -> &'static str {
        match self {
            McpTransportClient::Stdio(_) => "stdio",
            McpTransportClient::ChildProcess(_)
            | McpTransportClient::InterceptedChildProcess(_) => "child_process",

            #[cfg(feature = "http")]
            McpTransportClient::Http(_) | McpTransportClient::InterceptedHttp(_) => "http",

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(_) | McpTransportClient::InterceptedWebSocket(_) => {
                "websocket"
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(_) | McpTransportClient::InterceptedTcp(_) => "tcp",

            #[cfg(unix)]
            McpTransportClient::Unix(_) | McpTransportClient::InterceptedUnix(_) => "unix",
        }
    }

    /// Request completions from an MCP server (TurboMCP 1.0.10)
    /// This enables auto-completion for tool parameters and other inputs
    pub async fn complete(
        &self,
        completion_name: &str,
        partial_input: &str,
    ) -> Result<Vec<String>, McpError> {
        match self {
            McpTransportClient::Stdio(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }
            McpTransportClient::ChildProcess(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }

            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }

            #[cfg(unix)]
            McpTransportClient::Unix(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }

            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                let response = client.complete(completion_name, partial_input).await?;
                Ok(response.completion.values)
            }
        }
    }

    /// Register an elicitation handler for server user input requests (TurboMCP 2.0)
    /// Enables servers to request additional information from users during interactions
    #[allow(dead_code)]
    pub fn register_elicitation_handler(&self, handler: std::sync::Arc<dyn ElicitationHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.set_elicitation_handler(handler),
            McpTransportClient::ChildProcess(client) => client.set_elicitation_handler(handler),
            McpTransportClient::InterceptedChildProcess(client) => {
                client.set_elicitation_handler(handler)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.set_elicitation_handler(handler),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.set_elicitation_handler(handler),
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.set_elicitation_handler(handler),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.set_elicitation_handler(handler)
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.set_elicitation_handler(handler),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.set_elicitation_handler(handler),
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.set_elicitation_handler(handler),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.set_elicitation_handler(handler),
        }
    }

    /// Register a log handler for server log messages (TurboMCP 2.0)
    /// Routes server log messages to client logging system
    #[allow(dead_code)]
    pub fn register_log_handler(&self, handler: std::sync::Arc<dyn LogHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.set_log_handler(handler),
            McpTransportClient::ChildProcess(client) => client.set_log_handler(handler),
            McpTransportClient::InterceptedChildProcess(client) => client.set_log_handler(handler),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.set_log_handler(handler),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.set_log_handler(handler),
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.set_log_handler(handler),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => client.set_log_handler(handler),
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.set_log_handler(handler),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.set_log_handler(handler),
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.set_log_handler(handler),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.set_log_handler(handler),
        }
    }

    /// Register a resource update handler for resource change notifications (TurboMCP 2.0)
    /// Receives notifications when subscribed resources change on the server
    #[allow(dead_code)]
    pub fn register_resource_update_handler(
        &self,
        handler: std::sync::Arc<dyn ResourceUpdateHandler>,
    ) {
        match self {
            McpTransportClient::Stdio(client) => client.set_resource_update_handler(handler),
            McpTransportClient::ChildProcess(client) => client.set_resource_update_handler(handler),
            McpTransportClient::InterceptedChildProcess(client) => {
                client.set_resource_update_handler(handler)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.set_resource_update_handler(handler),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                client.set_resource_update_handler(handler)
            }
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.set_resource_update_handler(handler),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.set_resource_update_handler(handler)
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.set_resource_update_handler(handler),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                client.set_resource_update_handler(handler)
            }
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.set_resource_update_handler(handler),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                client.set_resource_update_handler(handler)
            }
        }
    }

    /// Check if an elicitation handler is registered (TurboMCP 2.0)
    pub fn has_elicitation_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_elicitation_handler(),
            McpTransportClient::ChildProcess(client) => client.has_elicitation_handler(),
            McpTransportClient::InterceptedChildProcess(client) => client.has_elicitation_handler(),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_elicitation_handler(),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.has_elicitation_handler(),
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_elicitation_handler(),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => client.has_elicitation_handler(),
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_elicitation_handler(),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.has_elicitation_handler(),
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.has_elicitation_handler(),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.has_elicitation_handler(),
        }
    }

    /// Check if a log handler is registered (TurboMCP 2.0)
    pub fn has_log_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_log_handler(),
            McpTransportClient::ChildProcess(client) => client.has_log_handler(),
            McpTransportClient::InterceptedChildProcess(client) => client.has_log_handler(),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_log_handler(),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.has_log_handler(),
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_log_handler(),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => client.has_log_handler(),
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_log_handler(),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.has_log_handler(),
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.has_log_handler(),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.has_log_handler(),
        }
    }

    /// Check if a resource update handler is registered (TurboMCP 2.0)
    pub fn has_resource_update_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_resource_update_handler(),
            McpTransportClient::ChildProcess(client) => client.has_resource_update_handler(),
            McpTransportClient::InterceptedChildProcess(client) => {
                client.has_resource_update_handler()
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_resource_update_handler(),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.has_resource_update_handler(),
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_resource_update_handler(),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.has_resource_update_handler()
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_resource_update_handler(),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.has_resource_update_handler(),
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.has_resource_update_handler(),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.has_resource_update_handler(),
        }
    }

    /// Subscribe to resource updates (transport-agnostic)
    pub async fn subscribe(&self, uri: &str) -> Result<(), McpError> {
        match self {
            McpTransportClient::Stdio(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }
            McpTransportClient::ChildProcess(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                client.subscribe(uri).await.map_err(|e| {
                    Self::http_error(
                        ErrorKind::Transport,
                        format!("HTTP subscribe failed: {}", e),
                    )
                })?;
                Ok(())
            }
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                client.subscribe(uri).await.map_err(|e| {
                    Self::http_error(
                        ErrorKind::Transport,
                        format!("HTTP subscribe failed: {}", e),
                    )
                })?;
                Ok(())
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }

            #[cfg(unix)]
            McpTransportClient::Unix(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                client.subscribe(uri).await?;
                Ok(())
            }
        }
    }

    /// Unsubscribe from resource updates (transport-agnostic)
    pub async fn unsubscribe(&self, uri: &str) -> Result<(), McpError> {
        match self {
            McpTransportClient::Stdio(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }
            McpTransportClient::ChildProcess(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                client.unsubscribe(uri).await.map_err(|e| {
                    Self::http_error(
                        ErrorKind::Transport,
                        format!("HTTP unsubscribe failed: {}", e),
                    )
                })?;
                Ok(())
            }
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                client.unsubscribe(uri).await.map_err(|e| {
                    Self::http_error(
                        ErrorKind::Transport,
                        format!("HTTP unsubscribe failed: {}", e),
                    )
                })?;
                Ok(())
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }

            #[cfg(unix)]
            McpTransportClient::Unix(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                client.unsubscribe(uri).await?;
                Ok(())
            }
        }
    }

    /// Register a progress handler for progress notifications
    #[allow(dead_code)]
    pub fn register_progress_handler(&self, handler: std::sync::Arc<dyn ProgressHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.set_progress_handler(handler),
            McpTransportClient::ChildProcess(client) => client.set_progress_handler(handler),
            McpTransportClient::InterceptedChildProcess(client) => {
                client.set_progress_handler(handler)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.set_progress_handler(handler),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.set_progress_handler(handler),
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.set_progress_handler(handler),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.set_progress_handler(handler)
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.set_progress_handler(handler),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.set_progress_handler(handler),
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.set_progress_handler(handler),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.set_progress_handler(handler),
        }
    }

    /// Check if a progress handler is registered
    #[allow(dead_code)]
    pub fn has_progress_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_progress_handler(),
            McpTransportClient::ChildProcess(client) => client.has_progress_handler(),
            McpTransportClient::InterceptedChildProcess(client) => client.has_progress_handler(),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_progress_handler(),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.has_progress_handler(),
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_progress_handler(),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => client.has_progress_handler(),
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_progress_handler(),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.has_progress_handler(),
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.has_progress_handler(),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.has_progress_handler(),
        }
    }

    /// Register a tool list changed handler
    #[allow(dead_code)]
    pub fn register_tool_list_changed_handler(
        &self,
        handler: std::sync::Arc<dyn ToolListChangedHandler>,
    ) {
        match self {
            McpTransportClient::Stdio(client) => client.set_tool_list_changed_handler(handler),
            McpTransportClient::ChildProcess(client) => {
                client.set_tool_list_changed_handler(handler)
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                client.set_tool_list_changed_handler(handler)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.set_tool_list_changed_handler(handler),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                client.set_tool_list_changed_handler(handler)
            }
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.set_tool_list_changed_handler(handler),
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.set_tool_list_changed_handler(handler)
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.set_tool_list_changed_handler(handler),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                client.set_tool_list_changed_handler(handler)
            }
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.set_tool_list_changed_handler(handler),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                client.set_tool_list_changed_handler(handler)
            }
        }
    }

    /// Register a prompt list changed handler
    #[allow(dead_code)]
    pub fn register_prompt_list_changed_handler(
        &self,
        handler: std::sync::Arc<dyn PromptListChangedHandler>,
    ) {
        match self {
            McpTransportClient::Stdio(client) => client.set_prompt_list_changed_handler(handler),
            McpTransportClient::ChildProcess(client) => {
                client.set_prompt_list_changed_handler(handler)
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                client.set_prompt_list_changed_handler(handler)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.set_prompt_list_changed_handler(handler),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                client.set_prompt_list_changed_handler(handler)
            }
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                client.set_prompt_list_changed_handler(handler)
            }
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.set_prompt_list_changed_handler(handler)
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.set_prompt_list_changed_handler(handler),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                client.set_prompt_list_changed_handler(handler)
            }
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.set_prompt_list_changed_handler(handler),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                client.set_prompt_list_changed_handler(handler)
            }
        }
    }

    /// Register a resource list changed handler
    #[allow(dead_code)]
    pub fn register_resource_list_changed_handler(
        &self,
        handler: std::sync::Arc<dyn ResourceListChangedHandler>,
    ) {
        match self {
            McpTransportClient::Stdio(client) => client.set_resource_list_changed_handler(handler),
            McpTransportClient::ChildProcess(client) => {
                client.set_resource_list_changed_handler(handler)
            }
            McpTransportClient::InterceptedChildProcess(client) => {
                client.set_resource_list_changed_handler(handler)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.set_resource_list_changed_handler(handler),
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => {
                client.set_resource_list_changed_handler(handler)
            }
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                client.set_resource_list_changed_handler(handler)
            }
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => {
                client.set_resource_list_changed_handler(handler)
            }
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.set_resource_list_changed_handler(handler),
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => {
                client.set_resource_list_changed_handler(handler)
            }
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.set_resource_list_changed_handler(handler),
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => {
                client.set_resource_list_changed_handler(handler)
            }
        }
    }

    /// Gracefully shutdown the MCP client and disconnect transport
    ///
    /// **CRITICAL**: Always call this before dropping the client to ensure:
    /// - WebSocket close frames are sent
    /// - Reconnection tasks are stopped
    /// - Background tasks are cleaned up
    /// - Resources are released properly
    ///
    /// Without calling shutdown(), WebSocket reconnection tasks will continue
    /// running even after the client is dropped!
    pub async fn shutdown(&self) -> Result<(), McpError> {
        match self {
            McpTransportClient::Stdio(client) => client.shutdown().await,
            McpTransportClient::ChildProcess(client) => client.shutdown().await,
            McpTransportClient::InterceptedChildProcess(client) => client.shutdown().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.shutdown().await,
            #[cfg(feature = "http")]
            McpTransportClient::InterceptedHttp(client) => client.shutdown().await,
            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.shutdown().await,
            #[cfg(feature = "websocket")]
            McpTransportClient::InterceptedWebSocket(client) => client.shutdown().await,
            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.shutdown().await,
            #[cfg(feature = "tcp")]
            McpTransportClient::InterceptedTcp(client) => client.shutdown().await,
            #[cfg(unix)]
            McpTransportClient::Unix(client) => client.shutdown().await,
            #[cfg(unix)]
            McpTransportClient::InterceptedUnix(client) => client.shutdown().await,
        }
    }
}
