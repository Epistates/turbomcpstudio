//! Managed MCP connection with health monitoring and metrics

use crate::mcp_client::process::ManagedProcess;
use crate::mcp_client::transport_client::McpTransportClient;
use crate::types::{ConnectionMetrics, ConnectionStatus, ServerCapabilities, ServerConfig};
use parking_lot::RwLock;
use std::time::Instant;

/// Snapshot of a connection's mutable state, acquired with a single lock pass per field.
///
/// Prefer this over accessing individual fields from multiple callers to reduce
/// lock contention and ensure a consistent view of connection state.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are public API; individual fields may not yet be read by all callers
pub struct ConnectionSnapshot {
    pub status: ConnectionStatus,
    pub request_count: u64,
    pub error_count: u64,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
    pub uptime_seconds: u64,
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

    /// Event sender for emitting connection events to frontend
    /// Enables reactive UI updates for capability changes, status changes, etc.
    pub event_sender: tokio::sync::mpsc::Sender<crate::mcp_client::events::ConnectionEvent>,

    /// Server ID for event emission
    pub server_id: uuid::Uuid,

    /// Tauri app handle for emitting protocol inspector events
    /// Used by the interceptor to emit intercepted messages to the frontend
    pub app_handle: tauri::AppHandle,
}

impl ManagedConnection {
    /// Acquire all locks once and return a plain-data snapshot of current connection state.
    ///
    /// Use this instead of acquiring individual locks in separate places to reduce
    /// contention and get a consistent point-in-time view.
    pub fn snapshot(&self) -> ConnectionSnapshot {
        let status = *self.status.read();
        let request_count = *self.request_count.lock();
        let error_count = *self.error_count.lock();
        let last_seen = *self.last_seen.read();
        let connected_instant = *self.connected_at.read();

        // Convert the Instant-based connected_at to a wall-clock DateTime
        let (connected_at, uptime_seconds) = match connected_instant {
            Some(instant) => {
                let uptime = instant.elapsed().as_secs();
                let wall_clock =
                    chrono::Utc::now() - chrono::Duration::seconds(uptime as i64);
                (Some(wall_clock), uptime)
            }
            None => (None, 0),
        };

        ConnectionSnapshot {
            status,
            request_count,
            error_count,
            connected_at,
            last_seen,
            uptime_seconds,
        }
    }
}
