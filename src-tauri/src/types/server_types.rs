//! MCP Studio Server and Transport Type Definitions
//!
//! This module defines all the types needed for server management, transport configuration,
//! and MCP protocol operations within MCP Studio.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use turbomcp_transport::{TransportType, TransportConfig as TurboTransportConfig};

// Re-export key TurboMCP types we need
pub use turbomcp_protocol::types::{
    ServerCapabilities, Tool, ToolInputSchema, PromptMessage,
    ContentBlock, TextContent, ImageContent, ResourceContent,
    ToolsCapabilities, PromptsCapabilities, ResourcesCapabilities
};

pub use turbomcp_transport::MessageDirection;
// NOTE: We define our own TransportConfig enum for UI needs, convert to TurboMCP struct when needed

/// Transport configuration for MCP Studio (UI-friendly enum)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TransportConfig {
    /// Standard I/O transport
    Stdio {
        command: String,
        args: Vec<String>,
        working_directory: Option<String>,
    },
    /// HTTP transport
    Http {
        url: String,
        headers: HashMap<String, String>,
    },
    /// WebSocket transport
    WebSocket {
        url: String,
        headers: HashMap<String, String>,
    },
    /// TCP socket transport
    Tcp {
        host: String,
        port: u16,
    },
    /// Unix domain socket transport
    Unix {
        path: String,
    },
}
// Note: TurboMCP's ServerInfo only has name, models, capabilities - we need our own for UI
// pub use turbomcp_client::sampling::ServerInfo;

/// Server information for MCP Studio UI (extends TurboMCP's minimal ServerInfo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Unique server identifier
    pub id: Uuid,
    /// Server configuration
    pub config: ServerConfig,
    /// Current connection status
    pub status: ConnectionStatus,
    /// Server capabilities
    pub capabilities: Option<ServerCapabilities>,
    /// Connection performance metrics
    pub metrics: ConnectionMetrics,
    /// Process information (for STDIO servers)
    pub process_info: Option<ProcessInfo>,
    /// When the server was last seen/contacted
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Server configuration for MCP Studio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Unique server identifier
    pub id: Uuid,
    /// Display name for the server
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Transport configuration
    pub transport_config: TransportConfig,
    /// Environment variables for the server process
    pub environment_variables: HashMap<String, String>,
    /// When this configuration was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this configuration was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Tool definition with schema information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (unique identifier)
    pub name: String,
    /// Human-readable title
    pub title: Option<String>,
    /// Description of what the tool does
    pub description: Option<String>,
    /// Input schema for parameters
    pub input_schema: ToolInputSchema,
    /// Optional output schema
    pub output_schema: Option<serde_json::Value>,
    /// Whether the tool is available
    pub available: bool,
}

/// Connection status for MCP servers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    /// Successfully connected and initialized
    Connected,
    /// Currently attempting to connect
    Connecting,
    /// Connection failed or lost
    Disconnected,
    /// Server encountered an error
    Error,
}

/// Connection performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    /// Time when connection was established
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Total number of requests sent
    pub requests_sent: u64,
    /// Total number of responses received
    pub responses_received: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Number of errors encountered
    pub error_count: u64,
    /// Last error message if any
    pub last_error: Option<String>,
    /// Bytes sent/received
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Process information for STDIO servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,
    /// Process status
    pub status: ProcessStatus,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// When the process was started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Command line that started the process
    pub command_line: String,
}

/// Status of a managed process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    /// Process is running normally
    Running,
    /// Process has stopped/exited
    Stopped,
    /// Process is in zombie state
    Zombie,
    /// Process status unknown
    Unknown,
}

/// Message history entry for protocol debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHistory {
    /// Unique message ID
    pub id: Uuid,
    /// Server this message was sent to/from
    pub server_id: Uuid,
    /// When the message was sent/received
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Direction of the message
    pub direction: MessageDirection,
    /// Raw message content
    pub content: String,
    /// Message size in bytes
    pub size_bytes: i64,
    /// Processing time in milliseconds (for responses)
    pub processing_time_ms: Option<i64>,
}

/// Execution status for scenarios and workflows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Not yet started
    Pending,
    /// Currently running
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self {
            connected_at: None,
            requests_sent: 0,
            responses_received: 0,
            avg_response_time_ms: 0.0,
            error_count: 0,
            last_error: None,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

impl Default for ProcessStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<Tool> for ToolDefinition {
    fn from(tool: Tool) -> Self {
        Self {
            name: tool.name,
            title: tool.title,
            description: tool.description,
            input_schema: tool.input_schema,
            output_schema: None, // Tool doesn't have output_schema field
            available: true,
        }
    }
}

/// Convert our TransportConfig enum to TurboMCP's TransportConfig struct
impl TransportConfig {
    pub fn to_turbo_config(&self) -> TurboTransportConfig {
        let transport_type = match self {
            TransportConfig::Stdio { .. } => TransportType::Stdio,
            TransportConfig::Http { .. } => TransportType::Http,
            TransportConfig::WebSocket { .. } => TransportType::WebSocket,
            TransportConfig::Tcp { .. } => TransportType::Tcp,
            TransportConfig::Unix { .. } => TransportType::Unix,
        };

        TurboTransportConfig {
            transport_type,
            connect_timeout: std::time::Duration::from_secs(30),
            read_timeout: None,
            write_timeout: None,
            keep_alive: None,
            max_connections: None,
            compression: false,
            compression_algorithm: None,
            custom: HashMap::new(),
        }
    }
}