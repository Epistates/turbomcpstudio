//! Transport-agnostic MCP client wrapper
//!
//! This module provides a unified interface for MCP clients across different
//! transport types (STDIO, HTTP, WebSocket, TCP, Unix sockets, etc.)

use std::collections::HashMap;
use std::sync::Arc;
use turbomcp_client::handlers::{
    ElicitationHandler, LogHandler, ProgressHandler, ResourceUpdateHandler,
};
use turbomcp_client::Client;
use turbomcp_protocol::types::{Prompt, PromptInput, Tool, ToolInputSchema};
use turbomcp_transport::child_process::ChildProcessTransport;
use turbomcp_transport::stdio::StdioTransport;

#[cfg(feature = "http")]
use turbomcp_transport::streamable_http_client::StreamableHttpClientTransport;

#[cfg(feature = "websocket")]
use turbomcp_transport::websocket::WebSocketTransport;

#[cfg(feature = "tcp")]
use turbomcp_transport::tcp::TcpTransport;

#[cfg(feature = "unix")]
use turbomcp_transport::unix::UnixTransport;

/// Transport-agnostic MCP client wrapper
///
/// Following TurboMCP's world-class Clone pattern (reqwest/AWS SDK style),
/// all clients are now cloneable directly - no SharedClient wrapper needed!
#[derive(Clone)]
pub enum McpTransportClient {
    #[allow(dead_code)]
    Stdio(Client<StdioTransport>),
    ChildProcess(Client<ChildProcessTransport>),

    #[cfg(feature = "http")]
    Http(Client<StreamableHttpClientTransport>),

    #[cfg(feature = "websocket")]
    WebSocket(Client<WebSocketTransport>),

    #[cfg(feature = "tcp")]
    Tcp(Client<TcpTransport>),

    #[cfg(feature = "unix")]
    Unix(Client<UnixTransport>),
}

impl McpTransportClient {
    /// Helper to convert HTTP string errors to TurboMCP Error
    #[cfg(feature = "http")]
    fn http_error(kind: turbomcp_core::ErrorKind, message: String) -> Box<turbomcp_core::Error> {
        // Error::new already returns Box<Error>
        turbomcp_core::Error::new(kind, message)
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
    /// This method is now an alias for list_tools() since TurboMCP 1.1.0
    /// returns full Tool objects with schemas by default
    pub async fn list_tools_with_schemas(&self) -> Result<Vec<Tool>, Box<turbomcp_core::Error>> {
        tracing::info!("✅ Getting tool schemas using TurboMCP 1.1.0 API");
        self.list_tools().await
    }

    /// Call a tool on the MCP server (transport-agnostic)
    pub async fn call_tool(
        &self,
        tool_name: &str,
        parameters: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<serde_json::Value, Box<turbomcp_core::Error>> {
        match self {
            McpTransportClient::Stdio(client) => client.call_tool(tool_name, parameters).await,
            McpTransportClient::ChildProcess(client) => {
                client.call_tool(tool_name, parameters).await
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                client.call_tool(tool_name, parameters).await.map_err(|e| {
                    Self::http_error(
                        turbomcp_core::ErrorKind::Transport,
                        format!("HTTP call_tool failed: {}", e),
                    )
                })
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.call_tool(tool_name, parameters).await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.call_tool(tool_name, parameters).await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.call_tool(tool_name, parameters).await,
        }
    }

    /// List tools available on the MCP server (transport-agnostic)
    pub async fn list_tools(&self) -> Result<Vec<Tool>, Box<turbomcp_core::Error>> {
        match self {
            McpTransportClient::Stdio(client) => client.list_tools().await,
            McpTransportClient::ChildProcess(client) => client.list_tools().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.list_tools().await.map_err(|e| {
                Self::http_error(
                    turbomcp_core::ErrorKind::Transport,
                    format!("HTTP list_tools failed: {}", e),
                )
            }),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.list_tools().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.list_tools().await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.list_tools().await,
        }
    }

    /// List prompts available on the MCP server (transport-agnostic)
    pub async fn list_prompts(&self) -> Result<Vec<Prompt>, Box<turbomcp_core::Error>> {
        match self {
            McpTransportClient::Stdio(client) => client.list_prompts().await,
            McpTransportClient::ChildProcess(client) => client.list_prompts().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.list_prompts().await.map_err(|e| {
                Self::http_error(
                    turbomcp_core::ErrorKind::Transport,
                    format!("HTTP list_prompts failed: {}", e),
                )
            }),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.list_prompts().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.list_prompts().await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.list_prompts().await,
        }
    }

    /// Get a specific prompt from the MCP server (transport-agnostic)
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<PromptInput>,
    ) -> Result<serde_json::Value, Box<turbomcp_core::Error>> {
        match self {
            McpTransportClient::Stdio(client) => {
                let result = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }
            McpTransportClient::ChildProcess(client) => {
                let result = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                let result = client.get_prompt(name, arguments).await.map_err(|e| {
                    Self::http_error(
                        turbomcp_core::ErrorKind::Transport,
                        format!("HTTP get_prompt failed: {}", e),
                    )
                })?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                let result = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => {
                let result = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => {
                let result = client.get_prompt(name, arguments).await?;
                Ok(serde_json::to_value(result)?)
            }
        }
    }

    /// List resources available on the MCP server (transport-agnostic)
    pub async fn list_resources(&self) -> Result<Vec<String>, Box<turbomcp_core::Error>> {
        match self {
            McpTransportClient::Stdio(client) => client.list_resources().await,
            McpTransportClient::ChildProcess(client) => client.list_resources().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.list_resources().await.map_err(|e| {
                Self::http_error(
                    turbomcp_core::ErrorKind::Transport,
                    format!("HTTP list_resources failed: {}", e),
                )
            }),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.list_resources().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.list_resources().await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.list_resources().await,
        }
    }

    /// Read a specific resource from the MCP server (transport-agnostic)
    pub async fn read_resource(
        &self,
        uri: &str,
    ) -> Result<serde_json::Value, Box<turbomcp_core::Error>> {
        match self {
            McpTransportClient::Stdio(client) => {
                let result = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }
            McpTransportClient::ChildProcess(client) => {
                let result = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => {
                let result = client.read_resource(uri).await.map_err(|e| {
                    Self::http_error(
                        turbomcp_core::ErrorKind::Transport,
                        format!("HTTP read_resource failed: {}", e),
                    )
                })?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => {
                let result = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => {
                let result = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => {
                let result = client.read_resource(uri).await?;
                Ok(serde_json::to_value(result)?)
            }
        }
    }

    /// Get the transport type for this client
    #[allow(dead_code)]
    pub fn transport_type(&self) -> &'static str {
        match self {
            McpTransportClient::Stdio(_) => "stdio",
            McpTransportClient::ChildProcess(_) => "child_process",

            #[cfg(feature = "http")]
            McpTransportClient::Http(_) => "http",

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(_) => "websocket",

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(_) => "tcp",

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(_) => "unix",
        }
    }

    /// Request completions from an MCP server (TurboMCP 1.0.10)
    /// This enables auto-completion for tool parameters and other inputs
    pub async fn complete(
        &self,
        completion_name: &str,
        partial_input: &str,
    ) -> Result<Vec<String>, Box<turbomcp_core::Error>> {
        match self {
            McpTransportClient::Stdio(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.completion.values),

            McpTransportClient::ChildProcess(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.completion.values),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.completion.values),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.completion.values),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.completion.values),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.completion.values),
        }
    }

    // REMOVED: list_roots() - TurboMCP 2.0 Breaking Change
    // Per MCP 2025-06-18 spec, roots/list is SERVER→CLIENT (not CLIENT→SERVER)
    // Servers request roots from clients, not vice versa
    // Clients should implement roots handler to respond to server requests

    /// Register an elicitation handler for server user input requests (TurboMCP 1.0.10)
    /// Enables servers to request additional information from users during interactions
    #[allow(dead_code)]
    pub async fn register_elicitation_handler(&self, handler: Arc<dyn ElicitationHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.on_elicitation(handler),

            McpTransportClient::ChildProcess(client) => client.on_elicitation(handler),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.on_elicitation(handler),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.on_elicitation(handler),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.on_elicitation(handler),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.on_elicitation(handler),
        }
    }

    /// Register a progress handler for server progress notifications (TurboMCP 1.0.10)
    /// Receives updates about long-running operations on the server
    #[allow(dead_code)]
    pub async fn register_progress_handler(&self, handler: Arc<dyn ProgressHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.on_progress(handler),

            McpTransportClient::ChildProcess(client) => client.on_progress(handler),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.on_progress(handler),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.on_progress(handler),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.on_progress(handler),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.on_progress(handler),
        }
    }

    /// Register a log handler for server log messages (TurboMCP 1.0.10)
    /// Routes server log messages to client logging system
    #[allow(dead_code)]
    pub async fn register_log_handler(&self, handler: Arc<dyn LogHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.on_log(handler),

            McpTransportClient::ChildProcess(client) => client.on_log(handler),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.on_log(handler),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.on_log(handler),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.on_log(handler),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.on_log(handler),
        }
    }

    /// Register a resource update handler for resource change notifications (TurboMCP 1.0.10)
    /// Receives notifications when subscribed resources change on the server
    #[allow(dead_code)]
    pub async fn register_resource_update_handler(&self, handler: Arc<dyn ResourceUpdateHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.on_resource_update(handler),

            McpTransportClient::ChildProcess(client) => client.on_resource_update(handler),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.on_resource_update(handler),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.on_resource_update(handler),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.on_resource_update(handler),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.on_resource_update(handler),
        }
    }

    /// Check if an elicitation handler is registered (TurboMCP 1.0.10)
    pub async fn has_elicitation_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_elicitation_handler(),

            McpTransportClient::ChildProcess(client) => client.has_elicitation_handler(),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_elicitation_handler(),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_elicitation_handler(),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_elicitation_handler(),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.has_elicitation_handler(),
        }
    }

    /// Check if a progress handler is registered (TurboMCP 1.0.10)
    pub async fn has_progress_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_progress_handler(),

            McpTransportClient::ChildProcess(client) => client.has_progress_handler(),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_progress_handler(),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_progress_handler(),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_progress_handler(),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.has_progress_handler(),
        }
    }

    /// Check if a log handler is registered (TurboMCP 1.0.10)
    pub async fn has_log_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_log_handler(),

            McpTransportClient::ChildProcess(client) => client.has_log_handler(),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_log_handler(),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_log_handler(),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_log_handler(),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.has_log_handler(),
        }
    }

    /// Check if a resource update handler is registered (TurboMCP 1.0.10)
    pub async fn has_resource_update_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_resource_update_handler(),

            McpTransportClient::ChildProcess(client) => client.has_resource_update_handler(),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_resource_update_handler(),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_resource_update_handler(),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_resource_update_handler(),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.has_resource_update_handler(),
        }
    }
}
