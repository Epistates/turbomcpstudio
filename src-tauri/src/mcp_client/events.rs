//! Connection events for UI updates and monitoring

use crate::types::{ConnectionMetrics, ConnectionStatus, ProcessInfo, ServerCapabilities};
use serde_json::Value;
use uuid::Uuid;

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
    /// Server log message received (from LogHandler)
    ServerLogMessage {
        server_id: Uuid,
        level: String,
        data: Value,
        logger: Option<String>,
    },
    /// Resource list changed on server (tools, prompts, or resources)
    ListChanged {
        server_id: Uuid,
        /// Which list changed: "tools", "prompts", or "resources"
        list_type: String,
    },
    /// A subscribed resource was updated
    ResourceUpdated {
        server_id: Uuid,
        uri: String,
    },
}
