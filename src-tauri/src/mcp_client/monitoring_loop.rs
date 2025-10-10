//! Monitoring Loop Module
//!
//! Provides background monitoring for MCP server connections with:
//! - Continuous process health monitoring
//! - Automatic health checks via MCP protocol
//! - Real-time metrics updates (uptime, process info)
//! - Connection status tracking and error recovery

use crate::error::McpResult;
use crate::mcp_client::connection::ManagedConnection;
use crate::mcp_client::events::ConnectionEvent;
use crate::types::ConnectionStatus;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Monitoring Loop Operations
///
/// Provides stateless operations for background monitoring tasks.
pub struct MonitoringLoop;

impl MonitoringLoop {
    /// Start enhanced background monitoring task with health checks and automatic recovery
    ///
    /// This spawns a tokio task that:
    /// 1. Updates process metrics every 5 seconds (CPU, memory, uptime)
    /// 2. Performs health checks every 30 seconds via MCP protocol
    /// 3. Broadcasts events for status changes and process updates
    /// 4. Automatically marks failed connections as errors
    pub fn start_monitoring(
        connections: Arc<DashMap<Uuid, Arc<ManagedConnection>>>,
        event_sender: mpsc::UnboundedSender<ConnectionEvent>,
        system: Arc<RwLock<System>>,
    ) -> tokio::task::JoinHandle<()> {
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
                                metrics.uptime_seconds = uptime_seconds;
                            }
                        }
                    }
                    _ = health_check_interval.tick() => {
                        // Perform health checks on connected servers
                        tracing::debug!("Performing health checks on {} connections", connections.len());

                        for entry in connections.iter() {
                            let server_id = *entry.key();
                            let connection = entry.value();
                            let status = *connection.status.read();

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
    ///
    /// Uses list_tools as a lightweight health check probe.
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

    /// Get process info by PID (static helper for background task)
    async fn get_process_info_by_pid_static(
        system: &Arc<RwLock<System>>,
        pid: u32,
    ) -> Option<crate::types::ProcessInfo> {
        use sysinfo::Pid;

        let system_guard = system.read();
        let sys_pid = Pid::from(pid as usize);

        system_guard.process(sys_pid).map(|proc| {
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

            crate::types::ProcessInfo {
                pid,
                command_line,
                started_at: chrono::Utc::now(), // TODO: Get actual start time if available
                cpu_usage: proc.cpu_usage() as f64,
                memory_usage: proc.memory(),
                status: crate::types::ProcessStatus::Running,
            }
        })
    }
}
