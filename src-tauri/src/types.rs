use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// World-class Collections system (enhanced)
pub mod collections;

/// Server configuration for connecting to MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub transport: TransportConfig,
    pub environment_variables: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Transport configuration for different MCP connection types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TransportConfig {
    Stdio {
        command: String,
        args: Vec<String>,
        working_directory: Option<String>,
    },
    Http {
        url: String,
        headers: HashMap<String, String>,
    },
    #[serde(rename = "websocket")]
    WebSocket {
        url: String,
        headers: HashMap<String, String>,
    },
    Tcp {
        host: String,
        port: u16,
    },
    Unix {
        path: String,
    },
}

/// MCP server connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Server runtime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: Uuid,
    pub config: ServerConfig,
    pub status: ConnectionStatus,
    pub capabilities: Option<ServerCapabilities>,
    pub process_info: Option<ProcessInfo>,
    pub metrics: ConnectionMetrics,
    pub last_seen: Option<DateTime<Utc>>,
}

/// MCP server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: Option<ToolsCapability>,
    pub resources: Option<ResourcesCapability>,
    pub prompts: Option<PromptsCapability>,
    pub sampling: Option<SamplingCapability>,
    pub elicitation: Option<ElicitationCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    pub subscribe: Option<bool>,
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationCapability {}

/// Process information for STDIO servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub args: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub status: ProcessStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Crashed(String),
}

/// Connection metrics and performance data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub avg_response_time_ms: f64,
    pub error_count: u64,
    pub last_error: Option<String>,
    pub uptime_seconds: u64,
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            avg_response_time_ms: 0.0,
            error_count: 0,
            last_error: None,
            uptime_seconds: 0,
        }
    }
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// MCP resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDefinition {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

// NOTE: Collection types now imported from collections module above
// Enhanced Collections system with cross-server workflows, variable passing,
// advanced assertions, and execution tracking.

/// Message history for protocol analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHistory {
    pub id: Uuid,
    pub server_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub direction: MessageDirection,
    pub content: serde_json::Value,
    pub size_bytes: usize,
    pub processing_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageDirection {
    Sent,
    Received,
}

/// Workspace for team collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub collections: Vec<Uuid>,
    pub members: Vec<WorkspaceMember>,
    pub settings: WorkspaceSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMember {
    pub user_id: String,
    pub role: WorkspaceRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    pub auto_save: bool,
    pub sync_enabled: bool,
    pub default_environment: Option<String>,
}
