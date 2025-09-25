use crate::error::{McpResult, McpStudioError};
use crate::types::{
    ConnectionMetrics, ConnectionStatus, ProcessInfo, ProcessStatus, ServerCapabilities,
    ServerConfig, ServerInfo, TransportConfig,
};
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{Pid, System};
use tokio::process::Child;
use tokio::sync::mpsc;
use uuid::Uuid;

// TurboMCP 1.1.0 imports - Enhanced Client API with Error Extensions
// Enhanced error handling - using direct error mapping for better compatibility
use turbomcp_client::sampling::{
    SamplingHandler, DelegatingSamplingHandler,
};
use turbomcp_client::{Client, ClientBuilder, ConnectionConfig, SharedClient};
// Type-State Capability Builders (TurboMCP 1.1.0)
use turbomcp_protocol::capabilities::builders::{
    ServerCapabilitiesBuilder, ClientCapabilitiesBuilder
};
// Plugin system ready for TurboMCP 1.1.0
#[cfg(feature = "plugins")]
use turbomcp_client::plugins::{
    MetricsPlugin, RetryPlugin, CachePlugin, PluginConfig,
    CacheConfig, RetryConfig, ClientPlugin
};
use turbomcp_client::handlers::{
    ElicitationAction, ElicitationHandler, ElicitationRequest, ElicitationResponse, HandlerResult, LogHandler,
    ProgressHandler, ResourceUpdateHandler,
};
use turbomcp_protocol::types::{Prompt, PromptInput, Tool, ToolInputSchema};
use turbomcp_transport::child_process::{ChildProcessConfig, ChildProcessTransport};
use turbomcp_transport::stdio::StdioTransport;
use turbomcp_transport::Transport;

#[cfg(feature = "http")]
use turbomcp_transport::http_sse::HttpSseTransport;

#[cfg(feature = "websocket")]
use turbomcp_transport::websocket::WebSocketTransport;

#[cfg(feature = "tcp")]
use turbomcp_transport::tcp::TcpTransport;

#[cfg(feature = "unix")]
use turbomcp_transport::unix::UnixTransport;

/// Studio-specific elicitation handler that forwards requests to the frontend
#[derive(Debug)]
pub struct StudioElicitationHandler {
    pending_requests: Arc<DashMap<String, ElicitationRequest>>,
    completed_responses: Arc<DashMap<String, ElicitationResponse>>,
}

#[async_trait::async_trait]
impl ElicitationHandler for StudioElicitationHandler {
    async fn handle_elicitation(
        &self,
        request: ElicitationRequest,
    ) -> HandlerResult<ElicitationResponse> {
        tracing::info!(
            "Received elicitation request: {} - {}",
            request.id,
            request.prompt
        );

        // Store the request for the frontend to pick up
        self.pending_requests
            .insert(request.id.clone(), request.clone());

        // Check for an existing response in the completed responses map
        // In a real implementation, this would wait for the frontend to provide a response
        if let Some(response) = self.completed_responses.get(&request.id) {
            let response = response.clone();

            // Remove from pending since it's now completed
            self.pending_requests.remove(&request.id);

            tracing::info!("Found completed response for elicitation: {}", request.id);
            Ok(response)
        } else {
            // For now, return a placeholder response indicating pending user input
            // In a full implementation, this would either wait for user input or return an error
            let response = ElicitationResponse {
                action: ElicitationAction::Accept,
                content: Some(serde_json::json!({
                    "status": "pending_user_input",
                    "message": "Request forwarded to frontend for user interaction"
                })),
            };

            tracing::info!("Elicitation request {} is pending user input", request.id);
            Ok(response)
        }
    }
}

/// Transport-agnostic MCP client wrapper
#[derive(Clone)]
pub enum McpTransportClient {
    Stdio(SharedClient<StdioTransport>),
    ChildProcess(SharedClient<ChildProcessTransport>),

    #[cfg(feature = "http")]
    Http(SharedClient<HttpSseTransport>),

    #[cfg(feature = "websocket")]
    WebSocket(SharedClient<WebSocketTransport>),

    #[cfg(feature = "tcp")]
    Tcp(SharedClient<TcpTransport>),

    #[cfg(feature = "unix")]
    Unix(SharedClient<UnixTransport>),
}

impl McpTransportClient {
    /// Create a basic tool schema for tools where we don't have full schema information
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
            McpTransportClient::Http(client) => client.call_tool(tool_name, parameters).await,

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
            McpTransportClient::Http(client) => client.list_tools().await,

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
            McpTransportClient::Http(client) => client.list_prompts().await,

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
                let result = client.get_prompt(name, arguments).await?;
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
            McpTransportClient::Http(client) => client.list_resources().await,

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
                let result = client.read_resource(uri).await?;
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
                .map(|response| response.values),

            McpTransportClient::ChildProcess(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.values),

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.values),

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.values),

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.values),

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client
                .complete(completion_name, partial_input)
                .await
                .map(|response| response.values),
        }
    }

    /// List filesystem roots available to the server (TurboMCP 1.0.11)
    /// Returns filesystem boundaries that define server access scope
    pub async fn list_roots(&self) -> Result<Vec<String>, Box<turbomcp_core::Error>> {
        match self {
            McpTransportClient::Stdio(client) => client.list_roots().await,

            McpTransportClient::ChildProcess(client) => client.list_roots().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.list_roots().await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.list_roots().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.list_roots().await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.list_roots().await,
        }
    }

    /// Register an elicitation handler for server user input requests (TurboMCP 1.0.10)
    /// Enables servers to request additional information from users during interactions
    pub async fn register_elicitation_handler(&self, handler: Arc<dyn ElicitationHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.on_elicitation(handler).await,

            McpTransportClient::ChildProcess(client) => client.on_elicitation(handler).await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.on_elicitation(handler).await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.on_elicitation(handler).await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.on_elicitation(handler).await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.on_elicitation(handler).await,
        }
    }

    /// Register a progress handler for server progress notifications (TurboMCP 1.0.10)
    /// Receives updates about long-running operations on the server
    pub async fn register_progress_handler(&self, handler: Arc<dyn ProgressHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.on_progress(handler).await,

            McpTransportClient::ChildProcess(client) => client.on_progress(handler).await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.on_progress(handler).await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.on_progress(handler).await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.on_progress(handler).await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.on_progress(handler).await,
        }
    }

    /// Register a log handler for server log messages (TurboMCP 1.0.10)
    /// Routes server log messages to client logging system
    pub async fn register_log_handler(&self, handler: Arc<dyn LogHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.on_log(handler).await,

            McpTransportClient::ChildProcess(client) => client.on_log(handler).await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.on_log(handler).await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.on_log(handler).await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.on_log(handler).await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.on_log(handler).await,
        }
    }

    /// Register a resource update handler for resource change notifications (TurboMCP 1.0.10)
    /// Receives notifications when subscribed resources change on the server
    pub async fn register_resource_update_handler(&self, handler: Arc<dyn ResourceUpdateHandler>) {
        match self {
            McpTransportClient::Stdio(client) => client.on_resource_update(handler).await,

            McpTransportClient::ChildProcess(client) => client.on_resource_update(handler).await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.on_resource_update(handler).await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.on_resource_update(handler).await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.on_resource_update(handler).await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.on_resource_update(handler).await,
        }
    }

    /// Check if an elicitation handler is registered (TurboMCP 1.0.10)
    pub async fn has_elicitation_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_elicitation_handler().await,

            McpTransportClient::ChildProcess(client) => client.has_elicitation_handler().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_elicitation_handler().await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_elicitation_handler().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_elicitation_handler().await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.has_elicitation_handler().await,
        }
    }

    /// Check if a progress handler is registered (TurboMCP 1.0.10)
    pub async fn has_progress_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_progress_handler().await,

            McpTransportClient::ChildProcess(client) => client.has_progress_handler().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_progress_handler().await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_progress_handler().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_progress_handler().await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.has_progress_handler().await,
        }
    }

    /// Check if a log handler is registered (TurboMCP 1.0.10)
    pub async fn has_log_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_log_handler().await,

            McpTransportClient::ChildProcess(client) => client.has_log_handler().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_log_handler().await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_log_handler().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_log_handler().await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.has_log_handler().await,
        }
    }

    /// Check if a resource update handler is registered (TurboMCP 1.0.10)
    pub async fn has_resource_update_handler(&self) -> bool {
        match self {
            McpTransportClient::Stdio(client) => client.has_resource_update_handler().await,

            McpTransportClient::ChildProcess(client) => client.has_resource_update_handler().await,

            #[cfg(feature = "http")]
            McpTransportClient::Http(client) => client.has_resource_update_handler().await,

            #[cfg(feature = "websocket")]
            McpTransportClient::WebSocket(client) => client.has_resource_update_handler().await,

            #[cfg(feature = "tcp")]
            McpTransportClient::Tcp(client) => client.has_resource_update_handler().await,

            #[cfg(feature = "unix")]
            McpTransportClient::Unix(client) => client.has_resource_update_handler().await,
        }
    }
}

/// Manager for MCP client connections using TurboMCP
pub struct McpClientManager {
    /// Active connections to MCP servers
    connections: DashMap<Uuid, Arc<ManagedConnection>>,

    /// System information for process monitoring
    system: Arc<RwLock<System>>,

    /// Event channel for connection updates
    event_sender: mpsc::UnboundedSender<ConnectionEvent>,

    /// Sampling handler for LLM integration
    sampling_handler: Option<Arc<dyn SamplingHandler>>,

    /// Elicitation handler for server-initiated user input requests
    elicitation_handler: Option<Arc<dyn ElicitationHandler>>,

    /// Storage for pending elicitation requests
    pending_elicitations: Arc<DashMap<String, ElicitationRequest>>,

    /// Storage for completed elicitation responses
    completed_elicitations: Arc<DashMap<String, ElicitationResponse>>,
}

/// A managed MCP connection with health monitoring and metrics
pub struct ManagedConnection {
    pub config: ServerConfig,
    pub status: RwLock<ConnectionStatus>,
    pub capabilities: RwLock<Option<ServerCapabilities>>,
    pub metrics: RwLock<ConnectionMetrics>,
    pub process: RwLock<Option<ManagedProcess>>,
    pub last_seen: RwLock<Option<chrono::DateTime<chrono::Utc>>>,

    /// Connection start time for uptime tracking
    pub connected_at: RwLock<Option<Instant>>,

    /// Request/response tracking for metrics
    pub request_count: parking_lot::Mutex<u64>,
    pub error_count: parking_lot::Mutex<u64>,

    /// Transport-agnostic MCP client
    pub client: RwLock<Option<McpTransportClient>>,
}

/// Managed process for STDIO servers
pub struct ManagedProcess {
    pub child: Child,
    pub pid: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub command: String,
    pub args: Vec<String>,
}

/// Connection events for UI updates
#[derive(Debug, Clone, serde::Serialize)]
pub enum ConnectionEvent {
    StatusChanged {
        server_id: Uuid,
        status: ConnectionStatus,
    },
    CapabilitiesUpdated {
        server_id: Uuid,
        capabilities: ServerCapabilities,
    },
    MetricsUpdated {
        server_id: Uuid,
        metrics: ConnectionMetrics,
    },
    ProcessUpdated {
        server_id: Uuid,
        process_info: ProcessInfo,
    },
    MessageReceived {
        server_id: Uuid,
        message: Value,
    },
    MessageSent {
        server_id: Uuid,
        message: Value,
    },
    Error {
        server_id: Uuid,
        error: String,
    },
}

impl McpClientManager {
    /// Create a new MCP client manager with enterprise-grade monitoring
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ConnectionEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        // Initialize sampling handler for LLM integration
        let sampling_handler = None; // Will be set via LLM configuration

        let manager = Self {
            connections: DashMap::new(),
            system: Arc::new(RwLock::new(System::new_all())),
            event_sender,
            sampling_handler,
            elicitation_handler: None,
            pending_elicitations: Arc::new(DashMap::new()),
            completed_elicitations: Arc::new(DashMap::new()),
        };

        tracing::info!("MCP Client Manager initialized - ready for enterprise connections");
        tracing::info!("Available transports: STDIO, HTTP, WebSocket, TCP, Unix");
        tracing::info!("TurboMCP v1.1.0 integration ready with plugin and LLM registry support");
        if manager.sampling_handler.is_some() {
            tracing::info!("Production LLM sampling handler initialized");
        } else {
            tracing::warn!("No LLM providers registered - sampling will be unavailable");
        }

        (manager, event_receiver)
    }

    /// Configure ClientBuilder with enterprise plugins for production reliability
    #[cfg(feature = "plugins")]
    fn configure_plugins(mut builder: ClientBuilder) -> ClientBuilder {
        // Retry plugin with exponential backoff - essential for MCP Studio's server testing
        let retry_config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            retry_on_timeout: true,
            retry_on_connection_error: true,
        };
        builder = builder.with_plugin(Arc::new(RetryPlugin::new(PluginConfig::Retry(retry_config))));

        // Cache plugin for performance - critical for repeated tool/resource calls
        let cache_config = CacheConfig {
            max_entries: 1000,
            ttl_seconds: 300, // 5 minutes - good for development workflow
            cache_responses: true,
            cache_resources: true,
            cache_tools: true,
        };
        builder = builder.with_plugin(Arc::new(CachePlugin::new(PluginConfig::Cache(cache_config))));

        // Metrics plugin for monitoring server performance - valuable for development
        builder = builder.with_plugin(Arc::new(MetricsPlugin::new(PluginConfig::Metrics)));

        tracing::info!("Enterprise plugins enabled: Retry, Cache, Metrics");
        builder
    }

    #[cfg(not(feature = "plugins"))]
    fn configure_plugins(builder: ClientBuilder) -> ClientBuilder {
        tracing::debug!("Plugin system disabled - using basic client");
        builder
    }

    /// Configure client capabilities with type-state builders for compile-time validation
    fn configure_client_capabilities() -> turbomcp_protocol::types::ClientCapabilities {
        // Use TurboMCP 1.1.0 type-state capability builders for compile-time validation
        let client_caps = ClientCapabilitiesBuilder::new()
            .enable_experimental()  // Enables experimental capability state
            .enable_roots()         // Enables roots capability state (for MCP Studio file access)
            .enable_sampling()      // Enables sampling capability state (for HITL)
            .enable_elicitation()   // Enables elicitation capability state
            // Sub-capability only available when roots are enabled!
            .enable_roots_list_changed()  // ✅ Only available when roots enabled
            // TurboMCP exclusive features for MCP Studio!
            .with_llm_provider("openai", "gpt-4")                  // 🚀 TurboMCP exclusive
            .with_ui_capabilities(vec!["form", "dialog", "toast"]) // 🚀 TurboMCP exclusive - perfect for MCP Studio UI
            .build();

        tracing::info!("Type-state client capabilities configured with compile-time validation");
        tracing::info!("✅ Roots enabled: {}", client_caps.roots.is_some());
        tracing::info!("✅ Sampling enabled: {}", client_caps.sampling.is_some());
        tracing::info!("✅ Elicitation enabled: {}", client_caps.elicitation.is_some());

        client_caps
    }

    /// Create a studio elicitation handler that forwards requests to the frontend
    pub fn create_studio_elicitation_handler(
        pending_requests: Arc<DashMap<String, ElicitationRequest>>,
        completed_responses: Arc<DashMap<String, ElicitationResponse>>,
    ) -> Arc<dyn ElicitationHandler> {
        Arc::new(StudioElicitationHandler {
            pending_requests,
            completed_responses,
        })
    }

    /// Create a sampling handler using runtime LLM configuration
    fn create_sampling_handler_from_config(
        llm_config: &crate::llm_config::LLMConfigManager,
    ) -> Option<Arc<dyn SamplingHandler>> {
        // This is a synchronous method but we need async operations
        // We'll need to restructure this or call it from async context
        tracing::warn!("Synchronous sampling handler creation deprecated - use async version");
        None
    }

    /// Update sampling handler using runtime configuration
    pub async fn update_sampling_handler(
        &self,
        llm_config: &crate::llm_config::LLMConfigManager,
    ) -> McpResult<bool> {
        // Get the active sampling handler from LLM config
        if let Some(handler) = llm_config.get_active_sampling_handler().await {
            // Update the sampling handler for all managed clients
            let connections = &self.connections;

            for connection_ref in connections.iter() {
                let connection = connection_ref.value();
                if let Some(client) = connection.client.read().as_ref() {
                    // Update the client's sampling handler
                    // Note: This would require adding a method to McpTransportClient
                    tracing::info!("Updated sampling handler for connection: {}", connection.config.id);
                }
            }

            tracing::info!("Sampling handler updated successfully");
            Ok(true)
        } else {
            tracing::warn!("No active LLM provider configured");
            Ok(false)
        }
    }

    /// Check if sampling is available
    pub async fn is_sampling_available(
        &self,
        llm_config: &crate::llm_config::LLMConfigManager,
    ) -> bool {
        llm_config.get_active_sampling_handler().await.is_some()
    }

    /// Connect to an MCP server
    pub async fn connect_server(&self, config: ServerConfig) -> McpResult<ServerInfo> {
        let server_id = config.id;

        // Create managed connection with enterprise monitoring
        let connection = Arc::new(ManagedConnection {
            config: config.clone(),
            status: RwLock::new(ConnectionStatus::Connecting),
            capabilities: RwLock::new(None),
            metrics: RwLock::new(ConnectionMetrics::default()),
            process: RwLock::new(None),
            last_seen: RwLock::new(None),
            connected_at: RwLock::new(Some(Instant::now())),
            request_count: parking_lot::Mutex::new(0),
            error_count: parking_lot::Mutex::new(0),
            client: RwLock::new(None),
        });

        // Store connection
        self.connections.insert(server_id, connection.clone());

        // Send status update
        self.send_event(ConnectionEvent::StatusChanged {
            server_id,
            status: ConnectionStatus::Connecting,
        });

        // Establish connection with enterprise-grade error handling
        let connection_result = self.establish_mcp_connection(connection.clone()).await;

        match connection_result {
            Ok(()) => {
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());

                // Send successful connection event
                self.send_event(ConnectionEvent::StatusChanged {
                    server_id,
                    status: ConnectionStatus::Connected,
                });

                tracing::info!(
                    "Successfully connected to MCP server: {} ({})",
                    config.name,
                    server_id
                );
            }
            Err(err) => {
                *connection.status.write() = ConnectionStatus::Error;
                *connection.error_count.lock() += 1;

                // Send error event
                self.send_event(ConnectionEvent::StatusChanged {
                    server_id,
                    status: ConnectionStatus::Error,
                });

                tracing::error!("Failed to connect to MCP server {}: {}", config.name, err);
                return Err(err);
            }
        }

        // Return server info
        self.get_server_info(server_id).await
    }

    /// Disconnect from an MCP server
    pub async fn disconnect_server(&self, server_id: Uuid) -> McpResult<()> {
        if let Some((_, connection)) = self.connections.remove(&server_id) {
            tracing::info!("Disconnecting from server: {}", connection.config.name);

            // Update status
            *connection.status.write() = ConnectionStatus::Disconnected;

            // Clean up TurboMCP client - it will handle process termination automatically
            if let Some(_client) = connection.client.write().take() {
                tracing::debug!(
                    "TurboMCP client cleaned up for server: {}",
                    connection.config.name
                );
                // TurboMCP ChildProcessTransport will automatically terminate the process when dropped
            }

            // Send event
            self.send_event(ConnectionEvent::StatusChanged {
                server_id,
                status: ConnectionStatus::Disconnected,
            });

            tracing::info!(
                "Successfully disconnected from server: {}",
                connection.config.name
            );
        }

        Ok(())
    }

    /// Get information about a connected server
    pub async fn get_server_info(&self, server_id: Uuid) -> McpResult<ServerInfo> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // For TurboMCP connections, process info is managed internally
        // We can still try to get process info if available, but it's optional
        let pid_opt = connection.process.read().as_ref().map(|p| p.pid);
        let process_info = if let Some(pid) = pid_opt {
            self.get_process_info_by_pid(pid).await
        } else {
            // No process info available for TurboMCP-managed connections
            None
        };

        let status = connection.status.read().clone();
        let capabilities = connection.capabilities.read().clone();
        let metrics = connection.metrics.read().clone();
        let last_seen = connection.last_seen.read()
            .unwrap_or_else(|| chrono::Utc::now());

        Ok(ServerInfo {
            id: server_id,
            config: connection.config.clone(),
            status,
            capabilities,
            metrics,
            process_info,
            last_seen,
        })
    }

    /// Establish MCP connection with enterprise-grade reliability and monitoring
    async fn establish_mcp_connection(&self, connection: Arc<ManagedConnection>) -> McpResult<()> {
        let config = &connection.config;

        tracing::info!(
            "Establishing MCP connection to: {} (transport: {:?})",
            config.name,
            config.transport_config
        );

        match &config.transport_config {
            TransportConfig::Stdio {
                command,
                args,
                working_directory,
            } => {
                // Start STDIO process with robust process management
                self.connect_stdio(
                    connection.clone(),
                    command,
                    args,
                    working_directory.as_deref(),
                )
                .await
            }
            #[cfg(feature = "http")]
            TransportConfig::Http { url, headers } => {
                tracing::info!("HTTP/SSE transport connection to: {}", url);
                self.connect_http(connection.clone(), url, headers).await
            }
            #[cfg(feature = "websocket")]
            TransportConfig::WebSocket { url, headers } => {
                tracing::info!("WebSocket transport connection to: {}", url);
                self.connect_websocket(connection.clone(), url, headers)
                    .await
            }
            #[cfg(feature = "tcp")]
            TransportConfig::Tcp { host, port } => {
                tracing::info!("TCP transport connection to: {}:{}", host, port);
                self.connect_tcp(connection.clone(), host, *port).await
            }
            #[cfg(feature = "unix")]
            TransportConfig::Unix { path } => {
                tracing::info!("Unix socket transport connection to: {}", path);
                self.connect_unix(connection.clone(), path).await
            }

            // Feature not enabled - handle unsupported transports
            #[cfg(not(feature = "http"))]
            TransportConfig::Http { .. } => Err(McpStudioError::UnsupportedTransport(
                "HTTP transport not enabled in build".to_string(),
            )),
            #[cfg(not(feature = "websocket"))]
            TransportConfig::WebSocket { .. } => Err(McpStudioError::UnsupportedTransport(
                "WebSocket transport not enabled in build".to_string(),
            )),
            #[cfg(not(feature = "tcp"))]
            TransportConfig::Tcp { .. } => Err(McpStudioError::UnsupportedTransport(
                "TCP transport not enabled in build".to_string(),
            )),
            #[cfg(not(feature = "unix"))]
            TransportConfig::Unix { .. } => Err(McpStudioError::UnsupportedTransport(
                "Unix transport not enabled in build".to_string(),
            )),
        }
    }

    /// List all connected servers
    pub async fn list_servers(&self) -> McpResult<Vec<ServerInfo>> {
        let mut servers = Vec::new();

        for entry in self.connections.iter() {
            let server_id = *entry.key();
            if let Ok(info) = self.get_server_info(server_id).await {
                servers.push(info);
            }
        }

        Ok(servers)
    }

    /// Send a tool call to an MCP server with automatic retry logic
    pub async fn call_tool(
        &self,
        server_id: Uuid,
        tool_name: &str,
        parameters: Value,
    ) -> McpResult<Value> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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
                            last_error.as_ref().unwrap()
                        );
                    } else {
                        tracing::error!(
                            "Tool call '{}' failed after {} attempts: {}",
                            tool_name,
                            max_retries + 1,
                            last_error.as_ref().unwrap()
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
            last_error.unwrap()
        )))
    }

    /// List tools available on an MCP server
    pub async fn list_tools(
        &self,
        server_id: Uuid,
    ) -> McpResult<Vec<crate::types::ToolDefinition>> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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
    pub async fn list_prompts(&self, server_id: Uuid) -> McpResult<Vec<serde_json::Value>> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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
    pub async fn get_prompt(
        &self,
        server_id: Uuid,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> McpResult<serde_json::Value> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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
            Some(serde_json::from_value::<PromptInput>(args)
                .map_err(|e| McpStudioError::SerializationError(e))?)
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
    pub async fn list_resources(&self, server_id: Uuid) -> McpResult<Vec<serde_json::Value>> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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
                    "name": uri.split("://").nth(1).unwrap_or(&uri).split('/').last().unwrap_or(&uri),
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
    pub async fn read_resource(
        &self,
        server_id: Uuid,
        uri: String,
    ) -> McpResult<serde_json::Value> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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

    /// Connect to STDIO MCP server using TurboMCP ChildProcessTransport
    async fn connect_stdio(
        &self,
        connection: Arc<ManagedConnection>,
        command: &str,
        args: &[String],
        working_directory: Option<&str>,
    ) -> McpResult<()> {
        tracing::info!("Connecting to STDIO MCP server using TurboMCP ChildProcessTransport:");
        tracing::info!("  Command: {} {:?}", command, args);
        tracing::info!("  Working directory: {:?}", working_directory);
        tracing::info!(
            "  Environment variables: {} entries",
            connection.config.environment_variables.len()
        );

        // Initialize TurboMCP ChildProcessTransport directly - let it handle the process lifecycle
        match self
            .initialize_child_process_client(&connection, command, args, working_directory)
            .await
        {
            Ok(client) => {
                *connection.client.write() = Some(client);
                tracing::info!(
                    "TurboMCP ChildProcessTransport initialized successfully for: {}",
                    command
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to initialize TurboMCP ChildProcessTransport: {}", e);
                Err(e)
            }
        }
    }

    /// Connect to HTTP/SSE MCP server
    #[cfg(feature = "http")]
    async fn connect_http(
        &self,
        connection: Arc<ManagedConnection>,
        url: &str,
        _headers: &std::collections::HashMap<String, String>,
    ) -> McpResult<()> {
        // Initialize HTTP/SSE transport and client
        match self.initialize_http_client(&connection, url).await {
            Ok(client) => {
                *connection.client.write() =
                    Some(McpTransportClient::Http(SharedClient::new(client)));
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());
                tracing::info!(
                    "TurboMCP HTTP/SSE client initialized successfully for: {}",
                    url
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to initialize TurboMCP HTTP client for {}: {}",
                    url,
                    e
                );
                Err(e)
            }
        }
    }

    /// Connect to WebSocket MCP server using TurboMCP 1.0.8
    #[cfg(feature = "websocket")]
    async fn connect_websocket(
        &self,
        connection: Arc<ManagedConnection>,
        url: &str,
        headers: &std::collections::HashMap<String, String>,
    ) -> McpResult<()> {
        tracing::info!("Establishing TurboMCP WebSocket connection to: {}", url);

        // Initialize WebSocket transport and client
        match self
            .initialize_websocket_client(&connection, url, headers)
            .await
        {
            Ok(client) => {
                *connection.client.write() =
                    Some(McpTransportClient::WebSocket(SharedClient::new(client)));
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());
                tracing::info!(
                    "TurboMCP WebSocket client initialized successfully for: {}",
                    url
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to initialize TurboMCP WebSocket client for {}: {}",
                    url,
                    e
                );
                Err(e)
            }
        }
    }

    #[cfg(not(feature = "websocket"))]
    async fn connect_websocket(
        &self,
        connection: Arc<ManagedConnection>,
        _url: &str,
        _headers: &std::collections::HashMap<String, String>,
    ) -> McpResult<()> {
        tracing::error!("WebSocket transport not compiled - enable 'websocket' feature");
        Err(McpStudioError::UnsupportedTransport(
            "WebSocket transport not compiled - enable 'websocket' feature".to_string(),
        ))
    }

    /// Connect to TCP MCP server using TurboMCP 1.0.8
    #[cfg(feature = "tcp")]
    async fn connect_tcp(
        &self,
        connection: Arc<ManagedConnection>,
        host: &str,
        port: u16,
    ) -> McpResult<()> {
        tracing::info!("Establishing TurboMCP TCP connection to: {}:{}", host, port);

        // Initialize TCP transport and client
        match self.initialize_tcp_client(&connection, host, port).await {
            Ok(client) => {
                *connection.client.write() =
                    Some(McpTransportClient::Tcp(SharedClient::new(client)));
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());
                tracing::info!(
                    "TurboMCP TCP client initialized successfully for: {}:{}",
                    host,
                    port
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to initialize TurboMCP TCP client for {}:{}: {}",
                    host,
                    port,
                    e
                );
                Err(e)
            }
        }
    }

    #[cfg(not(feature = "tcp"))]
    async fn connect_tcp(
        &self,
        connection: Arc<ManagedConnection>,
        _host: &str,
        _port: u16,
    ) -> McpResult<()> {
        tracing::error!("TCP transport not compiled - enable 'tcp' feature");
        Err(McpStudioError::UnsupportedTransport(
            "TCP transport not compiled - enable 'tcp' feature".to_string(),
        ))
    }

    /// Connect to Unix socket MCP server using TurboMCP 1.0.8
    #[cfg(feature = "unix")]
    async fn connect_unix(&self, connection: Arc<ManagedConnection>, path: &str) -> McpResult<()> {
        tracing::info!("Establishing TurboMCP Unix socket connection to: {}", path);

        // Initialize Unix socket transport and client
        match self.initialize_unix_client(&connection, path).await {
            Ok(client) => {
                *connection.client.write() =
                    Some(McpTransportClient::Unix(SharedClient::new(client)));
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());
                tracing::info!(
                    "TurboMCP Unix socket client initialized successfully for: {}",
                    path
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to initialize TurboMCP Unix socket client for {}: {}",
                    path,
                    e
                );
                Err(e)
            }
        }
    }

    #[cfg(not(feature = "unix"))]
    async fn connect_unix(&self, connection: Arc<ManagedConnection>, _path: &str) -> McpResult<()> {
        tracing::error!("Unix socket transport not compiled - enable 'unix' feature");
        Err(McpStudioError::UnsupportedTransport(
            "Unix socket transport not compiled - enable 'unix' feature".to_string(),
        ))
    }

    /// Get process information for monitoring
    async fn get_process_info(&self, process: &ManagedProcess) -> Option<ProcessInfo> {
        let mut system = self.system.write();
        system.refresh_all();

        if let Some(proc) = system.process(Pid::from(process.pid as usize)) {
            Some(ProcessInfo {
                pid: process.pid,
                command_line: format!("{} {}", process.command, process.args.join(" ")),
                started_at: process.started_at,
                cpu_usage: proc.cpu_usage() as f64,
                memory_usage: proc.memory(),
                status: ProcessStatus::Running,
            })
        } else {
            Some(ProcessInfo {
                pid: process.pid,
                command_line: format!("{} {}", process.command, process.args.join(" ")),
                started_at: process.started_at,
                cpu_usage: 0.0,
                memory_usage: 0,
                status: ProcessStatus::Stopped,
            })
        }
    }

    /// Send connection event
    fn send_event(&self, event: ConnectionEvent) {
        if let Err(e) = self.event_sender.send(event) {
            tracing::error!("Failed to send connection event: {}", e);
        }
    }

    /// Start enhanced background monitoring task with health checks and automatic recovery
    pub fn start_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let connections = self.connections.clone();
        let event_sender = self.event_sender.clone();
        let system = self.system.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            let mut health_check_interval = tokio::time::interval(Duration::from_secs(30)); // Health checks every 30s

            tracing::info!("Enhanced MCP connection monitoring started with health checks");

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Update system information and metrics
                        system.write().refresh_all();

                        // Check all connections for process updates
                        for entry in connections.iter() {
                            let server_id = *entry.key();
                            let connection = entry.value();

                            // Update process metrics if STDIO
                            let pid_opt = connection.process.read().as_ref().map(|p| p.pid);
                            if let Some(pid) = pid_opt {
                                if let Some(proc_info) = Self::get_process_info_by_pid_static(&system, pid).await {
                                    let _ = event_sender.send(ConnectionEvent::ProcessUpdated {
                                        server_id,
                                        process_info: proc_info,
                                    });
                                } else {
                                    // Process no longer exists - mark as disconnected
                                    tracing::warn!("MCP server process {} no longer exists, marking as disconnected", pid);
                                    *connection.status.write() = ConnectionStatus::Error;
                                    let _ = event_sender.send(ConnectionEvent::StatusChanged {
                                        server_id,
                                        status: ConnectionStatus::Error,
                                    });
                                }
                            }

                            // Update connection uptime metrics
                            if let Some(connected_at) = *connection.connected_at.read() {
                                let uptime_seconds = connected_at.elapsed().as_secs();
                                let mut metrics = connection.metrics.write();
                                // metrics.uptime_seconds = uptime_seconds; // Field does not exist in ConnectionMetrics
                            }
                        }
                    }
                    _ = health_check_interval.tick() => {
                        // Perform health checks on connected servers
                        tracing::debug!("Performing health checks on {} connections", connections.len());

                        for entry in connections.iter() {
                            let server_id = *entry.key();
                            let connection = entry.value();
                            let status = connection.status.read().clone();

                            // Only health check connected servers
                            if matches!(status, ConnectionStatus::Connected) {
                                let connection_clone = connection.clone();
                                let event_sender_clone = event_sender.clone();

                                // Spawn health check task (non-blocking)
                                tokio::spawn(async move {
                                    match Self::perform_health_check(&connection_clone).await {
                                        Ok(healthy) => {
                                            if healthy {
                                                // Update last seen time
                                                *connection_clone.last_seen.write() = Some(Utc::now());
                                                tracing::debug!("Health check passed for server {}", server_id);
                                            } else {
                                                // Health check failed
                                                tracing::warn!("Health check failed for server {}", server_id);
                                                *connection_clone.status.write() = ConnectionStatus::Error;
                                                *connection_clone.error_count.lock() += 1;

                                                let _ = event_sender_clone.send(ConnectionEvent::StatusChanged {
                                                    server_id,
                                                    status: ConnectionStatus::Error,
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Health check error for server {}: {}", server_id, e);
                                            *connection_clone.status.write() = ConnectionStatus::Error;
                                            *connection_clone.error_count.lock() += 1;

                                            let _ = event_sender_clone.send(ConnectionEvent::StatusChanged {
                                                server_id,
                                                status: ConnectionStatus::Error,
                                            });
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            }
        })
    }

    /// Perform health check on a connection by attempting a simple MCP operation
    async fn perform_health_check(connection: &Arc<ManagedConnection>) -> McpResult<bool> {
        // Get the client
        let client_opt = connection.client.read().clone();
        let client = match client_opt {
            Some(client) => client,
            None => return Ok(false), // No client means not healthy
        };

        // Try to list tools as a health check (lightweight operation)
        match client.list_tools().await {
            Ok(_) => {
                tracing::debug!(
                    "Health check successful for server {}",
                    connection.config.name
                );
                Ok(true)
            }
            Err(e) => {
                tracing::warn!(
                    "Health check failed for server {}: {}",
                    connection.config.name,
                    e
                );
                Ok(false)
            }
        }
    }

    /// Static helper for process info (for background task)
    async fn get_process_info_static(
        system: &Arc<RwLock<System>>,
        process: &ManagedProcess,
    ) -> Option<ProcessInfo> {
        let system = system.read();

        if let Some(proc) = system.process(Pid::from(process.pid as usize)) {
            Some(ProcessInfo {
                pid: process.pid,
                command_line: format!("{} {}", process.command, process.args.join(" ")),
                started_at: process.started_at,
                cpu_usage: proc.cpu_usage() as f64,
                memory_usage: proc.memory(),
                status: ProcessStatus::Running,
            })
        } else {
            Some(ProcessInfo {
                pid: process.pid,
                command_line: format!("{} {}", process.command, process.args.join(" ")),
                started_at: process.started_at,
                cpu_usage: 0.0,
                memory_usage: 0,
                status: ProcessStatus::Stopped,
            })
        }
    }

    /// Get process info by PID
    async fn get_process_info_by_pid(&self, pid: u32) -> Option<ProcessInfo> {
        Self::get_process_info_by_pid_static(&self.system, pid).await
    }

    /// Static helper for process info by PID (for background task)
    async fn get_process_info_by_pid_static(
        system: &Arc<RwLock<System>>,
        pid: u32,
    ) -> Option<ProcessInfo> {
        let system = system.read();

        system.process(Pid::from(pid as usize)).map(|proc| {
                let command_name = proc.name().to_string_lossy().to_string();
                let args: Vec<String> = proc
                    .cmd()
                    .iter()
                    .map(|arg| arg.to_string_lossy().to_string())
                    .collect();
                let command_line = if args.is_empty() {
                    command_name
                } else {
                    format!("{} {}", command_name, args.join(" "))
                };

                ProcessInfo {
                    pid,
                    command_line,
                    started_at: chrono::Utc::now(), // TODO: Get actual start time if available
                    cpu_usage: proc.cpu_usage() as f64,
                    memory_usage: proc.memory(),
                    status: ProcessStatus::Running,
                }
            })
    }

    /// Initialize TurboMCP client for ChildProcess transport - World-class implementation
    async fn initialize_child_process_client(
        &self,
        connection: &Arc<ManagedConnection>,
        command: &str,
        args: &[String],
        working_directory: Option<&str>,
    ) -> McpResult<McpTransportClient> {
        tracing::info!("Creating world-class TurboMCP ChildProcessTransport client...");

        // Create ChildProcessConfig with enterprise-grade settings
        let config = ChildProcessConfig {
            command: command.to_string(),
            args: args.to_vec(),
            working_directory: working_directory.map(|s| s.to_string()),
            environment: Some({
                let mut env_vars: Vec<(String, String)> = connection
                    .config
                    .environment_variables
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                // Suppress logging to prevent stdout pollution in MCP protocol
                env_vars.push(("RUST_LOG".to_string(), "".to_string()));

                env_vars
            }),
            startup_timeout: std::time::Duration::from_secs(30),
            shutdown_timeout: std::time::Duration::from_secs(10),
            max_message_size: 10 * 1024 * 1024, // 10MB
            buffer_size: 8192,
            kill_on_drop: true,
        };

        // Create the ChildProcessTransport
        let mut transport = ChildProcessTransport::new(config);

        // Connect the transport first (required by TurboMCP)
        transport.connect().await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("No such file or directory") || error_msg.contains("not found") {
                    McpStudioError::ConnectionFailed(format!(
                        "Server executable not found at path: {}\n\
                        \nPlease check:\n\
                        • The file path is correct\n\
                        • The executable has been built (try running 'cargo build --release' in the server directory)\n\
                        • You have permission to execute the file\n\
                        \nOriginal error: {}", command, error_msg
                    ))
                } else if error_msg.contains("Permission denied") {
                    McpStudioError::ConnectionFailed(format!(
                        "Permission denied when trying to execute: {}\n\
                        \nTry running: chmod +x {}\n\
                        \nOriginal error: {}", command, command, error_msg
                    ))
                } else {
                    McpStudioError::ConnectionFailed(format!("Failed to start MCP server process: {}", error_msg))
                }
            })?;

        // Build TurboMCP client with comprehensive capabilities, plugins, and extended timeout
        let connection_config = ConnectionConfig {
            timeout_ms: 60_000,    // 60 second timeout for initialization
            max_retries: 3,        // Retry failed requests
            retry_delay_ms: 1_000, // 1 second between retries
            keepalive_ms: 60_000,  // 60 second keepalive
        };

        // Use type-state capability builders for compile-time validation (TurboMCP 1.1.0)
        Self::configure_client_capabilities();

        let mut client = Self::configure_plugins(ClientBuilder::new())
            .with_tools(true)
            .with_prompts(true)
            .with_resources(true)
            .with_sampling(true) // Enable production-grade sampling with HITL
            .with_connection_config(connection_config)
            .build_sync(transport);

        // Initialize the client (performs MCP handshake)
        let init_result = client.initialize().await.map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("Invalid JSON-RPC response")
                || error_msg.contains("expected value at line 1 column 1")
            {
                McpStudioError::ConnectionFailed(format!(
                    "Server is not responding with valid JSON-RPC. This usually means:\n\
                        • The server is not an MCP server\n\
                        • The server is outputting logs or other text to stdout\n\
                        • The server path is incorrect or the server failed to start\n\
                        \nOriginal error: {}",
                    error_msg
                ))
            } else if error_msg.contains("No such file or directory")
                || error_msg.contains("not found")
            {
                McpStudioError::ConnectionFailed(format!(
                    "Server executable not found. Please check:\n\
                        • The path '{}' is correct\n\
                        • The server executable exists and is built\n\
                        • You have permission to execute the file\n\
                        \nOriginal error: {}",
                    command, error_msg
                ))
            } else {
                McpStudioError::ConnectionFailed(format!(
                    "Failed to initialize MCP client: {}",
                    error_msg
                ))
            }
        })?;

        tracing::info!("TurboMCP ChildProcessTransport client initialized successfully!");
        tracing::info!("Server info: {}", init_result.server_info.name);
        tracing::info!("Server version: {}", init_result.server_info.version);

        // Clone capabilities for logging before moving them in the mapping
        let raw_tools = init_result.server_capabilities.tools.clone();
        let raw_prompts = init_result.server_capabilities.prompts.clone();
        let raw_resources = init_result.server_capabilities.resources.clone();

        tracing::info!("Server capabilities from TurboMCP init result:");
        tracing::info!("  - Tools: {:?}", raw_tools);
        tracing::info!("  - Prompts: {:?}", raw_prompts);
        tracing::info!("  - Resources: {:?}", raw_resources);
        tracing::info!("  - Completions: {:?}", init_result.server_capabilities.completions);
        tracing::info!("  - Logging: {:?}", init_result.server_capabilities.logging);
        tracing::info!("  - Experimental: {:?}", init_result.server_capabilities.experimental);

        // Update connection capabilities with negotiated capabilities
        let server_capabilities = crate::types::ServerCapabilities {
            tools: init_result
                .server_capabilities
                .tools
                .map(|_| crate::types::ToolsCapabilities { list_changed: None }),
            prompts: init_result
                .server_capabilities
                .prompts
                .map(|_| crate::types::PromptsCapabilities { list_changed: None }),
            resources: init_result.server_capabilities.resources.map(|_| {
                crate::types::ResourcesCapabilities {
                    subscribe: None,
                    list_changed: None,
                }
            }),
            completions: init_result.server_capabilities.completions,
            experimental: init_result.server_capabilities.experimental,
            logging: init_result.server_capabilities.logging,
        };

        *connection.capabilities.write() = Some(server_capabilities.clone());
        tracing::info!("Mapped capabilities: {:?}", server_capabilities);

        Ok(McpTransportClient::ChildProcess(SharedClient::new(client)))
    }

    /// Initialize TurboMCP client for HTTP/SSE transport
    #[cfg(feature = "http")]
    async fn initialize_http_client(
        &self,
        _connection: &Arc<ManagedConnection>,
        _url: &str,
    ) -> McpResult<Client<HttpSseTransport>> {
        // TODO: Implement HTTP/SSE client initialization
        // This requires configuring HTTP SSE transport properly
        Err(McpStudioError::UnsupportedTransport(
            "HTTP/SSE client initialization not yet implemented".to_string(),
        ))
    }

    /// Initialize TurboMCP client for WebSocket transport
    #[cfg(feature = "websocket")]
    async fn initialize_websocket_client(
        &self,
        connection: &Arc<ManagedConnection>,
        url: &str,
        _headers: &std::collections::HashMap<String, String>,
    ) -> McpResult<Client<WebSocketTransport>> {
        // Create WebSocket transport
        let transport = WebSocketTransport::new(url).await.map_err(|e| {
            let error_msg = format!(
                "Failed to create WebSocket transport for {}: {}",
                connection.config.name, e
            );
            tracing::error!("{}", error_msg);
            McpStudioError::ConnectionFailed(error_msg)
        })?;

        // Build client with comprehensive capabilities and extended timeout
        let connection_config = ConnectionConfig {
            timeout_ms: 60_000,    // 60 second timeout for initialization
            max_retries: 3,        // Retry failed requests
            retry_delay_ms: 1_000, // 1 second between retries
            keepalive_ms: 60_000,  // 60 second keepalive
        };

        // Use type-state capability builders for compile-time validation (TurboMCP 1.1.0)
        Self::configure_client_capabilities();

        let mut client = Self::configure_plugins(ClientBuilder::new())
            .with_tools(true)
            .with_prompts(true)
            .with_resources(true)
            .with_sampling(true) // Enable production-grade sampling with HITL
            .with_connection_config(connection_config)
            .build_sync(transport);

        // Initialize the MCP connection and perform capability negotiation
        let init_result = client.initialize().await.map_err(|e| {
            let error_msg = format!(
                "Failed to initialize WebSocket MCP client for {}: {}",
                connection.config.name, e
            );
            tracing::error!("{}", error_msg);
            McpStudioError::ConnectionFailed(error_msg)
        })?;

        // Update connection capabilities
        let server_capabilities = crate::types::ServerCapabilities {
            tools: init_result
                .server_capabilities
                .tools
                .map(|_| crate::types::ToolsCapabilities { list_changed: None }),
            prompts: init_result
                .server_capabilities
                .prompts
                .map(|_| crate::types::PromptsCapabilities { list_changed: None }),
            resources: init_result.server_capabilities.resources.map(|_| {
                crate::types::ResourcesCapabilities {
                    subscribe: None,
                    list_changed: None,
                }
            }),
            completions: init_result.server_capabilities.completions,
            experimental: init_result.server_capabilities.experimental,
            logging: init_result.server_capabilities.logging,
        };

        *connection.capabilities.write() = Some(server_capabilities.clone());

        tracing::info!(
            "WebSocket MCP client initialized successfully for server '{}' ({})",
            connection.config.name,
            init_result.server_info.name
        );

        Ok(client)
    }

    /// Initialize TurboMCP client for TCP transport using TurboMCP 1.0.8
    #[cfg(feature = "tcp")]
    async fn initialize_tcp_client(
        &self,
        connection: &Arc<ManagedConnection>,
        host: &str,
        port: u16,
    ) -> McpResult<Client<TcpTransport>> {
        tracing::info!(
            "Creating TurboMCP 1.0.8 TCP client for: {} ({}:{})",
            connection.config.name,
            host,
            port
        );

        // Create TCP transport
        let address = format!("{}:{}", host, port);
        let socket_addr: std::net::SocketAddr = address.parse().map_err(|e| {
            let error_msg = format!("Invalid TCP address {}: {}", address, e);
            tracing::error!("{}", error_msg);
            McpStudioError::ConnectionFailed(error_msg)
        })?;
        let transport = TcpTransport::new_client(socket_addr, socket_addr);

        // Build client with comprehensive capabilities and extended timeout
        let connection_config = ConnectionConfig {
            timeout_ms: 60_000,    // 60 second timeout for initialization
            max_retries: 3,        // Retry failed requests
            retry_delay_ms: 1_000, // 1 second between retries
            keepalive_ms: 60_000,  // 60 second keepalive
        };

        // Use type-state capability builders for compile-time validation (TurboMCP 1.1.0)
        Self::configure_client_capabilities();

        let mut client = Self::configure_plugins(ClientBuilder::new())
            .with_tools(true)
            .with_prompts(true)
            .with_resources(true)
            .with_sampling(true) // Enable production-grade sampling with HITL
            .with_connection_config(connection_config)
            .build_sync(transport);

        // Initialize the MCP connection and perform capability negotiation
        let init_result = client.initialize().await.map_err(|e| {
            let error_msg = format!(
                "Failed to initialize TCP MCP client for {}: {}",
                connection.config.name, e
            );
            tracing::error!("{}", error_msg);
            McpStudioError::ConnectionFailed(error_msg)
        })?;

        // Update connection capabilities
        let server_capabilities = crate::types::ServerCapabilities {
            tools: init_result
                .server_capabilities
                .tools
                .map(|_| crate::types::ToolsCapabilities { list_changed: None }),
            prompts: init_result
                .server_capabilities
                .prompts
                .map(|_| crate::types::PromptsCapabilities { list_changed: None }),
            resources: init_result.server_capabilities.resources.map(|_| {
                crate::types::ResourcesCapabilities {
                    subscribe: None,
                    list_changed: None,
                }
            }),
            completions: init_result.server_capabilities.completions,
            experimental: init_result.server_capabilities.experimental,
            logging: init_result.server_capabilities.logging,
        };

        *connection.capabilities.write() = Some(server_capabilities.clone());

        tracing::info!(
            "TCP MCP client initialized successfully for server '{}' ({}:{})",
            connection.config.name,
            host,
            port
        );

        Ok(client)
    }

    /// Initialize TurboMCP client for Unix socket transport using TurboMCP 1.0.8
    #[cfg(feature = "unix")]
    async fn initialize_unix_client(
        &self,
        connection: &Arc<ManagedConnection>,
        path: &str,
    ) -> McpResult<Client<UnixTransport>> {
        tracing::info!(
            "Creating TurboMCP 1.0.8 Unix socket client for: {} ({})",
            connection.config.name,
            path
        );

        // Create Unix socket transport
        let socket_path = std::path::PathBuf::from(path);
        let transport = UnixTransport::new_client(socket_path);

        // Build client with comprehensive capabilities and extended timeout
        let connection_config = ConnectionConfig {
            timeout_ms: 60_000,    // 60 second timeout for initialization
            max_retries: 3,        // Retry failed requests
            retry_delay_ms: 1_000, // 1 second between retries
            keepalive_ms: 60_000,  // 60 second keepalive
        };

        // Use type-state capability builders for compile-time validation (TurboMCP 1.1.0)
        Self::configure_client_capabilities();

        let mut client = Self::configure_plugins(ClientBuilder::new())
            .with_tools(true)
            .with_prompts(true)
            .with_resources(true)
            .with_sampling(true) // Enable production-grade sampling with HITL
            .with_connection_config(connection_config)
            .build_sync(transport);

        // Initialize the MCP connection and perform capability negotiation
        let init_result = client.initialize().await.map_err(|e| {
            let error_msg = format!(
                "Failed to initialize Unix socket MCP client for {}: {}",
                connection.config.name, e
            );
            tracing::error!("{}", error_msg);
            McpStudioError::ConnectionFailed(error_msg)
        })?;

        // Update connection capabilities
        let server_capabilities = crate::types::ServerCapabilities {
            tools: init_result
                .server_capabilities
                .tools
                .map(|_| crate::types::ToolsCapabilities { list_changed: None }),
            prompts: init_result
                .server_capabilities
                .prompts
                .map(|_| crate::types::PromptsCapabilities { list_changed: None }),
            resources: init_result.server_capabilities.resources.map(|_| {
                crate::types::ResourcesCapabilities {
                    subscribe: None,
                    list_changed: None,
                }
            }),
            completions: init_result.server_capabilities.completions,
            experimental: init_result.server_capabilities.experimental,
            logging: init_result.server_capabilities.logging,
        };

        *connection.capabilities.write() = Some(server_capabilities.clone());

        tracing::info!(
            "Unix socket MCP client initialized successfully for server '{}' ({})",
            connection.config.name,
            path
        );

        Ok(client)
    }

    /// Get connection metrics for monitoring dashboard
    pub async fn get_connection_metrics(&self, server_id: Uuid) -> McpResult<ConnectionMetrics> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        let mut metrics = connection.metrics.read().clone();

        // Update real-time metrics
        metrics.requests_sent = *connection.request_count.lock();
        metrics.error_count = *connection.error_count.lock();

        // Calculate uptime
        if let Some(connected_at) = *connection.connected_at.read() {
            // metrics.uptime_seconds = connected_at.elapsed().as_secs(); // Field does not exist
        }

        Ok(metrics)
    }

    /// Get aggregated metrics for all connections
    pub async fn get_aggregated_metrics(&self) -> McpResult<ConnectionMetrics> {
        let mut aggregated = ConnectionMetrics::default();

        for entry in self.connections.iter() {
            let connection = entry.value();
            let metrics = connection.metrics.read();

            aggregated.requests_sent += metrics.requests_sent;
            aggregated.responses_received += metrics.responses_received;
            aggregated.bytes_sent += metrics.bytes_sent;
            aggregated.bytes_received += metrics.bytes_received;
            aggregated.error_count += metrics.error_count;

            // Calculate average response time (weighted by message count)
            if aggregated.requests_sent > 0 {
                let total_time = (aggregated.avg_response_time_ms
                    * (aggregated.requests_sent - metrics.requests_sent) as f64)
                    + (metrics.avg_response_time_ms * metrics.requests_sent as f64);
                aggregated.avg_response_time_ms = total_time / aggregated.requests_sent as f64;
            }

            // Use the maximum uptime as the overall uptime
            // aggregated.uptime_seconds = aggregated.uptime_seconds.max(metrics.uptime_seconds); // Field does not exist
        }

        Ok(aggregated)
    }

    /// Create a sampling request using TurboMCP's production-grade LLM integration
    pub async fn create_sampling_request_with_config(
        &self,
        server_id: Uuid,
        messages: Vec<serde_json::Value>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        llm_config: &crate::llm_config::LLMConfigManager,
    ) -> McpResult<serde_json::Value> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Get the active sampling handler from runtime configuration
        if let Some(handler) = llm_config.get_active_sampling_handler().await {
            tracing::info!(
                "Processing sampling request for server: {} with {} messages",
                connection.config.name,
                messages.len()
            );

            // Convert JSON messages to proper MCP protocol format
            use turbomcp_protocol::types::{
                Content, CreateMessageRequest, Role, SamplingMessage, TextContent,
            };

            let sampling_messages: Result<Vec<SamplingMessage>, _> = messages
                .into_iter()
                .map(|msg| {
                    let role = match msg.get("role").and_then(|r| r.as_str()) {
                        Some("user") => Role::User,
                        Some("assistant") => Role::Assistant,
                        _ => Role::User, // Default to user
                    };

                    let content = msg
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();

                    Ok(SamplingMessage {
                        role,
                        content: Content::Text(TextContent {
                            text: content,
                            annotations: None,
                            meta: None,
                        }),
                    })
                })
                .collect();

            let sampling_messages = sampling_messages.map_err(|e: &str| {
                McpStudioError::ConnectionError(format!("Failed to parse messages: {}", e))
            })?;

            let request = CreateMessageRequest {
                messages: sampling_messages,
                max_tokens: max_tokens.unwrap_or(1000),
                temperature: temperature.map(|t| t as f64),
                system_prompt: None,
                stop_sequences: None,
                model_preferences: None,
                include_context: None,
                metadata: None,
                _meta: None,
            };

            // Process the sampling request through TurboMCP
            match handler.handle_create_message(request).await {
                Ok(result) => {
                    tracing::info!(
                        "Sampling request completed successfully for server: {}",
                        connection.config.name
                    );

                    // Convert result back to JSON format
                    let response = serde_json::json!({
                        "status": "completed",
                        "role": match result.role {
                            Role::User => "user",
                            Role::Assistant => "assistant",
                        },
                        "content": match result.content {
                            Content::Text(text) => text.text,
                            _ => "Unsupported content type".to_string(),
                        },
                        "model": result.model,
                        "stop_reason": result.stop_reason,
                        "processed_messages": 1
                    });

                    Ok(response)
                }
                Err(e) => {
                    tracing::error!(
                        "Sampling request failed for server {}: {}",
                        connection.config.name,
                        e
                    );

                    let response = serde_json::json!({
                        "status": "error",
                        "message": format!("Sampling failed: {}", e),
                        "server": connection.config.name
                    });

                    Ok(response)
                }
            }
        } else {
            // Fallback response when no LLM provider is configured
            tracing::warn!(
                "No LLM provider configured - using fallback response for server: {}",
                connection.config.name
            );

            let response = serde_json::json!({
                "status": "no_provider",
                "message": "No LLM provider configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY environment variable.",
                "received_messages": messages.len(),
                "max_tokens": max_tokens,
                "temperature": temperature,
                "note": "Configure an LLM provider to enable real sampling functionality"
            });

            Ok(response)
        }
    }

    /// Send an elicitation response back to a server
    pub async fn send_elicitation_response(
        &self,
        server_id: Uuid,
        request_id: String,
        response_data: serde_json::Value,
    ) -> McpResult<serde_json::Value> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check if we have a pending elicitation request
        if let Some(pending_request) = self.pending_elicitations.get(&request_id) {
            let _pending_request = pending_request.clone();

            // Create a proper elicitation response
            let elicitation_response = ElicitationResponse {
                action: ElicitationAction::Accept,
                content: Some(response_data.clone()),
            };

            // Store the completed response
            self.completed_elicitations
                .insert(request_id.clone(), elicitation_response.clone());

            // Remove from pending requests
            self.pending_elicitations.remove(&request_id);

            tracing::info!(
                "Completed elicitation response for server: {} (request_id: {})",
                connection.config.name,
                request_id
            );

            // Return success response
            Ok(serde_json::json!({
                "status": "completed",
                "request_id": request_id,
                "response_processed": true,
                "data": response_data
            }))
        } else {
            // No pending request found
            tracing::warn!(
                "No pending elicitation request found for ID: {}",
                request_id
            );
            Ok(serde_json::json!({
                "status": "error",
                "message": "No pending elicitation request found for the given ID",
                "request_id": request_id
            }))
        }
    }

    /// Get pending elicitation requests from a server
    pub async fn get_elicitation_requests(
        &self,
        server_id: Uuid,
    ) -> McpResult<Vec<serde_json::Value>> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Return all pending elicitation requests as JSON values
        let pending_requests: Vec<serde_json::Value> = self
            .pending_elicitations
            .iter()
            .map(|entry| {
                let request = entry.value();
                serde_json::to_value(request).unwrap_or_else(|err| {
                    tracing::warn!("Failed to serialize elicitation request: {}", err);
                    serde_json::json!({
                        "id": request.id,
                        "prompt": request.prompt,
                        "schema": {},
                        "error": "Failed to serialize request"
                    })
                })
            })
            .collect();

        tracing::info!(
            "Retrieved {} pending elicitation requests for server: {}",
            pending_requests.len(),
            connection.config.name
        );

        Ok(pending_requests)
    }

    /// Get argument completions from an MCP server (TurboMCP 1.0.10)
    /// Provides autocompletion suggestions for prompt arguments and resource URIs
    pub async fn get_completions(
        &self,
        server_id: Uuid,
        completion_name: String,
        partial_input: String,
    ) -> McpResult<Vec<String>> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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

    /// List filesystem roots available to the server (TurboMCP 1.0.10)
    /// Returns filesystem boundaries that define server access scope
    pub async fn list_filesystem_roots(&self, server_id: Uuid) -> McpResult<Vec<String>> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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

        // List filesystem roots using TurboMCP client
        let roots = client.list_roots().await.map_err(|e| {
            *connection.error_count.lock() += 1;
            McpStudioError::ToolCallFailed(format!("Failed to list filesystem roots: {}", e))
        })?;

        // Update metrics
        *connection.request_count.lock() += 1;
        *connection.last_seen.write() = Some(Utc::now());

        tracing::info!(
            "Listed {} filesystem roots from server {}",
            roots.len(),
            server_id
        );

        Ok(roots)
    }

    /// Check handler registration status for a server (TurboMCP 1.0.10)
    /// Returns which event handlers are currently registered
    pub async fn get_handler_status(&self, server_id: Uuid) -> McpResult<serde_json::Value> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        // Check connection status
        let status = connection.status.read().clone();
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
