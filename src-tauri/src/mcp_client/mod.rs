//! MCP Client Module - Modular Architecture
//!
//! This module provides a comprehensive, enterprise-grade MCP (Model Context Protocol) client
//! implementation with support for multiple transports, health monitoring, and advanced features.
//!
//! ## Architecture
//!
//! The module is organized into focused sub-modules for better maintainability:
//!
//! - **connection** - Managed connection state with health monitoring and metrics
//! - **elicitation** - Server-initiated user input request handling
//! - **events** - Connection events for UI updates and monitoring
//! - **manager** - Main orchestrator for MCP server connections
//! - **process** - Child process management for STDIO servers
//! - **transport_client** - Transport-agnostic client wrapper for all MCP operations
//!
//! ## Features
//!
//! - **Multi-Transport Support**: STDIO, HTTP/SSE, WebSocket, TCP, Unix sockets
//! - **Enterprise Reliability**: Connection pooling, retry logic, circuit breakers
//! - **TurboMCP Integration**: Full 2.0 API with SIMD-accelerated JSON processing
//! - **Health Monitoring**: Process tracking, resource usage, connection metrics
//! - **Event-Driven**: Real-time updates for UI and protocol inspection
//! - **Handler Support**: Elicitation, sampling, progress, logging, resource updates
//!
//! ## Usage
//!
//! ```rust
//! use crate::mcp_client::McpClientManager;
//!
//! let (manager, event_receiver) = McpClientManager::new(app_handle);
//! let server_id = manager.connect_server(config).await?;
//! let tools = manager.list_tools(server_id).await?;
//! ```

use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

// Module declarations
mod connection;
mod elicitation;
mod events;
pub mod manager;
mod process;
mod sampling;
mod transport_client;

// Re-export main types for convenient access
pub use manager::McpClientManager;
// Note: StudioElicitationHandler and StudioSamplingHandler
// are only used internally in manager.rs, no need to re-export

/// Server context information for attribution in handlers
///
/// This struct contains metadata about which MCP server initiated a request.
/// Used with task-local storage to properly attribute sampling and elicitation
/// requests in multi-server scenarios.
#[derive(Debug, Clone)]
pub struct ServerContext {
    pub server_id: Uuid,
    pub server_name: String,
    pub server_description: Option<String>,
    pub connected_at: DateTime<Utc>,
}

impl Default for ServerContext {
    fn default() -> Self {
        Self {
            server_id: Uuid::nil(),
            server_name: "Unknown Server".to_string(),
            server_description: Some("Context unavailable".to_string()),
            connected_at: Utc::now(),
        }
    }
}

// Task-local storage for server context
//
// This allows handlers (sampling, elicitation) to determine which server
// made the request without requiring changes to TurboMCP's trait signatures.
//
// Set this before delegating to handlers, read it within handler implementations.
tokio::task_local! {
    pub static CURRENT_SERVER_CONTEXT: Arc<ServerContext>;
}
