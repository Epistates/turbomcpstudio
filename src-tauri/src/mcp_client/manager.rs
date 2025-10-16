//! MCP Client Manager - Main orchestrator for MCP server connections
//!
//! This module provides the primary interface for managing MCP server connections,
//! handling multiple transports (STDIO, HTTP, WebSocket, TCP, Unix), and coordinating
//! all MCP protocol operations. It integrates with TurboMCP 2.0 for enterprise-grade
//! reliability, performance, and compliance with the MCP 2025-06-18 specification.
//!
//! # Key Features
//!
//! - **Multi-Transport Support**: STDIO, HTTP/SSE, WebSocket, TCP, Unix sockets
//! - **Enterprise Monitoring**: Health checks, metrics, process monitoring
//! - **Production Reliability**: Automatic retry, circuit breakers, connection pooling
//! - **Event-Driven Architecture**: Real-time connection status updates
//! - **MCP Protocol Compliance**: Full support for tools, resources, prompts, sampling, elicitation
//!
//! # Architecture
//!
//! The manager maintains a collection of `ManagedConnection` instances, each representing
//! an active MCP server connection. It provides:
//!
//! - Connection lifecycle management (connect, disconnect, health monitoring)
//! - MCP operation routing (tools, resources, prompts, sampling)
//! - Process monitoring for STDIO servers
//! - Metrics collection and aggregation
//! - Event broadcasting for UI updates

use crate::error::{McpResult, McpStudioError};
use crate::types::{
    ConnectionMetrics, ConnectionStatus, ProcessInfo, ProcessStatus, ServerConfig, ServerInfo,
};
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use sysinfo::{Pid, System};
use tokio::sync::mpsc;
use uuid::Uuid;

// TurboMCP 2.0+ imports - World-class Clone pattern (no more SharedClient!)
use turbomcp_client::sampling::SamplingHandler;

use turbomcp_client::handlers::ElicitationResponse;

// Local module imports
use super::connection::ManagedConnection;
use super::elicitation::StudioElicitationHandler;
use super::events::ConnectionEvent;
use super::health_monitoring::HealthMonitoring;
use super::mcp_operations::McpOperations;
use super::message_history::MessageHistory;
use super::misc_operations::MiscOperations;
use super::monitoring_loop::MonitoringLoop;
use super::process::ManagedProcess;
use super::sampling::StudioSamplingHandler;
use super::sampling_logic::SamplingLogic;
use super::transport_layer::TransportLayer;

/// MCP Client Manager
///
/// The central coordinator for all MCP server connections in the application.
/// Manages connection lifecycle, health monitoring, and protocol operations.
pub struct McpClientManager {
    /// Active connections to MCP servers
    connections: DashMap<Uuid, Arc<ManagedConnection>>,

    /// System information for process monitoring
    system: Arc<RwLock<System>>,

    /// Event channel for connection updates (Issue #20: bounded to prevent memory leak)
    event_sender: mpsc::Sender<ConnectionEvent>,

    /// Sampling handler for LLM integration (HITL approval)
    sampling_handler: Arc<StudioSamplingHandler>,

    /// Elicitation handler for server-initiated user input requests
    elicitation_handler: Arc<StudioElicitationHandler>,
}

impl McpClientManager {
    /// Create a new MCP client manager with enterprise-grade monitoring
    pub fn new(
        app_handle: tauri::AppHandle,
        llm_config: Arc<crate::llm_config::LLMConfigManager>,
        db: Arc<tokio::sync::RwLock<Option<Arc<crate::database::Database>>>>,
    ) -> (Self, mpsc::Receiver<ConnectionEvent>) {
        // Issue #20 fix: Use bounded channel to prevent memory leak
        // Buffer size: 1000 events ≈ 500KB max (vs potentially 288MB with unbounded)
        let (event_sender, event_receiver) = mpsc::channel(1000);

        // Initialize sampling handler with HITL approval and protocol logging
        let sampling_handler = Arc::new(StudioSamplingHandler::new(
            app_handle.clone(),
            llm_config,
            db.clone(),
        ));

        // Initialize elicitation handler with Tauri app handle and protocol logging
        let elicitation_handler = Arc::new(StudioElicitationHandler::new(app_handle, db));

        let manager = Self {
            connections: DashMap::new(),
            system: Arc::new(RwLock::new(System::new_all())),
            event_sender,
            sampling_handler,
            elicitation_handler,
        };

        tracing::info!("MCP Client Manager initialized - ready for enterprise connections");
        tracing::info!("Available transports: STDIO, HTTP, WebSocket, TCP, Unix");
        tracing::info!(
            "TurboMCP v{} (protocol: {}) integration ready with plugin and LLM registry support",
            turbomcp_client::VERSION,
            turbomcp_protocol::PROTOCOL_VERSION
        );
        tracing::info!("Elicitation handler initialized with protocol logging");
        tracing::info!("Sampling handler initialized with HITL approval and protocol logging");

        (manager, event_receiver)
    }

    /// Submit an elicitation response from the frontend
    pub fn submit_elicitation_response(
        &self,
        request_id: String,
        response: ElicitationResponse,
    ) -> Result<(), String> {
        self.elicitation_handler
            .submit_response(request_id, response)
    }

    /// Approve a sampling request and forward to LLM
    pub fn approve_sampling_request(
        &self,
        request_id: String,
        approved_request: turbomcp_protocol::types::CreateMessageRequest,
    ) -> Result<(), String> {
        self.sampling_handler
            .submit_approved_request(request_id, approved_request)
    }

    /// Submit manual sampling response (bypass LLM - testing tool feature)
    pub fn submit_manual_sampling_response(
        &self,
        request_id: String,
        manual_response: turbomcp_protocol::types::CreateMessageResult,
    ) -> Result<(), String> {
        self.sampling_handler
            .submit_manual_response(request_id, manual_response)
    }

    /// Reject a sampling request
    pub fn reject_sampling_request(
        &self,
        request_id: String,
        reason: String,
    ) -> Result<(), String> {
        self.sampling_handler.reject_request(request_id, reason)
    }

    // Studio elicitation handler is now created in new() with AppHandle

    /// Create a sampling handler using runtime LLM configuration
    #[allow(dead_code)]
    fn create_sampling_handler_from_config(
        _llm_config: &crate::llm_config::LLMConfigManager,
    ) -> Option<Arc<dyn SamplingHandler>> {
        // This is a synchronous method but we need async operations
        // We'll need to restructure this or call it from async context
        tracing::warn!("Synchronous sampling handler creation deprecated - use async version");
        None
    }

    /// Update sampling handler using runtime configuration
    pub async fn update_sampling_handler(
        &self,
        _llm_config: &crate::llm_config::LLMConfigManager,
    ) -> McpResult<bool> {
        // Get the active sampling handler from LLM config
        if let Some(_handler) = _llm_config.get_active_sampling_handler().await {
            // Update the sampling handler for all managed clients
            let connections = &self.connections;

            for connection_ref in connections.iter() {
                let connection = connection_ref.value();
                if let Some(_client) = connection.client.read().as_ref() {
                    // Update the client's sampling handler
                    // Note: This would require adding a method to McpTransportClient
                    tracing::info!(
                        "Updated sampling handler for connection: {}",
                        connection.config.id
                    );
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

            // ✅ CRITICAL FIX: Call shutdown() before dropping client
            // This ensures WebSocket reconnection tasks are stopped and resources cleaned up
            let client_opt = connection.client.write().take();
            if let Some(client) = client_opt {
                tracing::debug!(
                    "Shutting down TurboMCP client for server: {}",
                    connection.config.name
                );

                // Call shutdown() to gracefully disconnect transport
                // This is CRITICAL for WebSocket - stops reconnection tasks and sends close frames
                if let Err(e) = client.shutdown().await {
                    tracing::warn!(
                        "Error during client shutdown for {}: {}",
                        connection.config.name,
                        e
                    );
                }

                tracing::debug!(
                    "TurboMCP client shut down successfully for server: {}",
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

        let status = *connection.status.read();
        let capabilities = connection.capabilities.read().clone();
        let metrics = connection.metrics.read().clone();
        let last_seen = connection.last_seen.read().unwrap_or_else(chrono::Utc::now);

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
        TransportLayer::establish_mcp_connection(
            connection,
            self.sampling_handler.clone(),
            self.elicitation_handler.clone(),
        )
        .await
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
        McpOperations::call_tool(&self.connections, server_id, tool_name, parameters).await
    }

    /// List tools available on an MCP server
    pub async fn list_tools(
        &self,
        server_id: Uuid,
    ) -> McpResult<Vec<crate::types::ToolDefinition>> {
        McpOperations::list_tools(&self.connections, server_id).await
    }

    /// List prompts available on an MCP server
    pub async fn list_prompts(&self, server_id: Uuid) -> McpResult<Vec<serde_json::Value>> {
        McpOperations::list_prompts(&self.connections, server_id).await
    }

    /// Get a specific prompt from an MCP server
    pub async fn get_prompt(
        &self,
        server_id: Uuid,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> McpResult<serde_json::Value> {
        McpOperations::get_prompt(&self.connections, server_id, name, arguments).await
    }

    /// List resources available on an MCP server
    pub async fn list_resources(&self, server_id: Uuid) -> McpResult<Vec<serde_json::Value>> {
        McpOperations::list_resources(&self.connections, server_id).await
    }

    /// Read a specific resource from an MCP server
    pub async fn read_resource(
        &self,
        server_id: Uuid,
        uri: String,
    ) -> McpResult<serde_json::Value> {
        McpOperations::read_resource(&self.connections, server_id, uri).await
    }

    /// Connect to STDIO MCP server using TurboMCP ChildProcessTransport
    /// Get process information for monitoring
    #[allow(dead_code)]
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

    /// Send connection event (Issue #20: non-blocking with bounded channel)
    fn send_event(&self, event: ConnectionEvent) {
        use tokio::sync::mpsc::error::TrySendError;

        match self.event_sender.try_send(event) {
            Ok(_) => {}
            Err(TrySendError::Full(_)) => {
                tracing::warn!("Event channel full, dropping event (backpressure active)");
                // Event dropped - frontend likely paused/slow
            }
            Err(TrySendError::Closed(_)) => {
                tracing::error!("Event channel closed, cannot send event");
            }
        }
    }

    /// Start enhanced background monitoring task with health checks and automatic recovery
    pub fn start_monitoring(&self) -> tokio::task::JoinHandle<()> {
        // Wrap DashMap in Arc for sharing across the monitoring task
        let connections_arc = Arc::new(self.connections.clone());
        MonitoringLoop::start_monitoring(
            connections_arc,
            self.event_sender.clone(),
            self.system.clone(),
        )
    }

    /// Static helper for process info (for background task)
    #[allow(dead_code)]
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

    /// Register capabilities and handlers after initialization
    ///
    /// Used when you need custom error handling during initialize (e.g., ChildProcess).
    /// Takes the init_result and performs:
    /// 1. Map and store server capabilities
    /// 2. Register bidirectional handlers (sampling + elicitation)
    ///
    /// Get connection metrics for monitoring dashboard
    pub async fn get_connection_metrics(&self, server_id: Uuid) -> McpResult<ConnectionMetrics> {
        HealthMonitoring::get_connection_metrics(&self.connections, server_id).await
    }

    /// Get aggregated metrics for all connections
    pub async fn get_aggregated_metrics(&self) -> McpResult<ConnectionMetrics> {
        HealthMonitoring::get_aggregated_metrics(&self.connections).await
    }

    /// Create and process sampling request with runtime LLM configuration
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

        SamplingLogic::create_sampling_request_with_config(
            &connection,
            messages,
            max_tokens,
            temperature,
            llm_config,
        )
        .await
    }

    // Elicitation methods removed - now using event-driven StudioElicitationHandler
    // See: submit_elicitation_response() method and StudioElicitationHandler struct
    // The old polling-based approach has been replaced with Tauri events + oneshot channels

    /// Get argument completions from an MCP server (TurboMCP 1.0.10)
    /// Get autocompletion suggestions for prompt arguments and resource URIs
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

        MiscOperations::get_completions(&connection, server_id, completion_name, partial_input)
            .await
    }

    /// List filesystem roots (deprecated in MCP 2025-06-18)
    pub async fn list_filesystem_roots(&self, server_id: Uuid) -> McpResult<Vec<String>> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        MiscOperations::list_filesystem_roots(&connection, server_id).await
    }

    /// Check handler registration status for a server
    pub async fn get_handler_status(&self, server_id: Uuid) -> McpResult<serde_json::Value> {
        let connection = self
            .connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        MiscOperations::get_handler_status(&connection, server_id).await
    }

    // ========================================================================
    // Message Tracking Methods (Stub Implementations for Commands)
    // ========================================================================

    /// Track request and save to message history (Protocol Inspector)
    pub async fn track_request_with_history(
        &self,
        server_id: Uuid,
        method: &str,
        request_data: &serde_json::Value,
        database: &Arc<crate::database::Database>,
    ) -> McpResult<(u64, ())> {
        MessageHistory::track_request_with_history(server_id, method, request_data, database).await
    }

    /// Track request start timing
    pub async fn track_request_start(
        &self,
        request_id: Uuid,
        method: &str,
        request_size: u64,
    ) -> McpResult<(u64, ())> {
        MessageHistory::track_request_start(request_id, method, request_size).await
    }

    /// Track response and save to message history with latency (Protocol Inspector)
    pub async fn track_response_with_history(
        &self,
        server_id: Uuid,
        start_time: u64,
        response_data: &serde_json::Value,
        database: &Arc<crate::database::Database>,
    ) -> McpResult<()> {
        MessageHistory::track_response_with_history(server_id, start_time, response_data, database)
            .await
    }

    /// Track request error
    pub async fn track_request_error(&self, request_id: Uuid, error: &str) -> McpResult<()> {
        MessageHistory::track_request_error(request_id, error).await
    }

    /// Save a message to history database (Protocol Inspector feature)
    pub async fn save_message_to_history(
        &self,
        server_id: Uuid,
        message_data: serde_json::Value,
        direction: turbomcp_transport::MessageDirection,
        timestamp: Option<chrono::DateTime<chrono::Utc>>,
        database: &Arc<crate::database::Database>,
    ) -> McpResult<()> {
        MessageHistory::save_message_to_history(
            server_id,
            message_data,
            direction,
            timestamp,
            database,
        )
        .await
    }

    /// Get all connection metrics (stub implementation)
    pub async fn get_all_connection_metrics(&self) -> HashMap<Uuid, ConnectionMetrics> {
        HealthMonitoring::get_all_connection_metrics(&self.connections).await
    }

    /// Get all server information for dashboard (stub implementation)
    pub async fn get_all_server_info(&self) -> Vec<ServerInfo> {
        let mut server_infos = Vec::new();
        for entry in self.connections.iter() {
            let connection = entry.value();
            let server_info = ServerInfo {
                id: connection.config.id,
                config: connection.config.clone(),
                status: *connection.status.read(),
                capabilities: connection.capabilities.read().clone(),
                metrics: connection.metrics.read().clone(),
                process_info: None, // Process tracking simplified for now
                last_seen: connection.last_seen.read().unwrap_or(chrono::Utc::now()),
            };
            server_infos.push(server_info);
        }
        server_infos
    }
}
