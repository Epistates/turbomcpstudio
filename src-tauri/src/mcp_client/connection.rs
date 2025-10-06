//! Managed MCP connection with health monitoring and metrics

use crate::mcp_client::process::ManagedProcess;
use crate::mcp_client::transport_client::McpTransportClient;
use crate::types::{ConnectionMetrics, ConnectionStatus, ServerCapabilities, ServerConfig};
use parking_lot::RwLock;
use std::time::Instant;

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
