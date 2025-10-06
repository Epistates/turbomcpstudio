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
}
