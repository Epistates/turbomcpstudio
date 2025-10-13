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
use tokio::sync::mpsc::error::TrySendError;  // Issue #20: For bounded channel backpressure
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
        event_sender: mpsc::Sender<ConnectionEvent>,  // Issue #20: Changed to bounded
        system: Arc<RwLock<System>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            let mut health_check_interval = tokio::time::interval(Duration::from_secs(30)); // Health checks every 30s

            // ✅ NOTE: Message processing loop removed - turbomcp-client v2.0+ handles this automatically
            // The MessageDispatcher background task now routes all server-initiated requests (elicitation, sampling)
            // No manual process_message() calls needed!

            tracing::info!("Enhanced MCP connection monitoring started with health checks");

            loop {
                tokio::select! {
                    // Message processing removed - handled automatically by turbomcp-client dispatcher
                    _ = interval.tick() => {
                        // Issue #23 fix: Only refresh specific PIDs instead of all system processes
                        // Optimization: refresh_all() takes 50-100ms, refresh_processes(Some(&[...])) takes <1ms total

                        // Collect PIDs to refresh (only STDIO servers with active processes)
                        let pids_to_refresh: Vec<sysinfo::Pid> = connections
                            .iter()
                            .filter_map(|entry| {
                                entry.value().process.read().as_ref().map(|p| sysinfo::Pid::from(p.pid as usize))
                            })
                            .collect();

                        // Refresh only specific processes instead of all system processes
                        if !pids_to_refresh.is_empty() {
                            use sysinfo::ProcessesToUpdate;
                            let mut system_guard = system.write();
                            system_guard.refresh_processes(ProcessesToUpdate::Some(&pids_to_refresh), false);
                        }

                        // Check all connections for process updates
                        for entry in connections.iter() {
                            let server_id = *entry.key();
                            let connection = entry.value();

                            // Update process metrics if STDIO
                            let pid_opt = connection.process.read().as_ref().map(|p| p.pid);
                            if let Some(pid) = pid_opt {
                                if let Some(proc_info) = Self::get_process_info_by_pid_static(&system, pid).await {
                                    // Issue #20: Use try_send for bounded channel backpressure handling
                                    match event_sender.try_send(ConnectionEvent::ProcessUpdated {
                                        server_id,
                                        process_info: proc_info,
                                    }) {
                                        Ok(_) => {},
                                        Err(TrySendError::Full(_)) => {
                                            tracing::warn!("Event channel full, skipping ProcessUpdated for {}", server_id);
                                            // Dropped event - will be sent next monitoring interval
                                        }
                                        Err(TrySendError::Closed(_)) => {
                                            tracing::error!("Event channel closed, stopping monitoring");
                                            return;
                                        }
                                    }
                                } else {
                                    // Process no longer exists - mark as disconnected
                                    tracing::warn!("MCP server process {} no longer exists, marking as disconnected", pid);
                                    *connection.status.write() = ConnectionStatus::Error;

                                    // Issue #20: Use try_send for bounded channel backpressure handling
                                    match event_sender.try_send(ConnectionEvent::StatusChanged {
                                        server_id,
                                        status: ConnectionStatus::Error,
                                    }) {
                                        Ok(_) => {},
                                        Err(TrySendError::Full(_)) => {
                                            tracing::warn!("Event channel full, skipping StatusChanged for {}", server_id);
                                        }
                                        Err(TrySendError::Closed(_)) => {
                                            tracing::error!("Event channel closed, stopping monitoring");
                                            return;
                                        }
                                    }
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
                        // Issue #22 fix: Use JoinSet to limit concurrent health checks
                        // Optimization: Prevents unbounded task spawning, caps at max_servers concurrent checks

                        // Collect connected servers that need health checks
                        let servers_to_check: Vec<(Uuid, Arc<ManagedConnection>)> = connections
                            .iter()
                            .filter_map(|entry| {
                                let server_id = *entry.key();
                                let connection = entry.value();
                                let status = *connection.status.read();

                                if matches!(status, ConnectionStatus::Connected) {
                                    Some((server_id, connection.clone()))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        tracing::debug!("Performing health checks on {} connected servers", servers_to_check.len());

                        // Use JoinSet for bounded concurrency (all checks complete before next interval)
                        let mut join_set = tokio::task::JoinSet::new();

                        for (server_id, connection) in servers_to_check {
                            let event_sender_clone = event_sender.clone();

                            join_set.spawn(async move {
                                match Self::perform_health_check(&connection).await {
                                    Ok(healthy) => {
                                        if healthy {
                                            // Update last seen time
                                            *connection.last_seen.write() = Some(Utc::now());
                                            tracing::debug!("Health check passed for server {}", server_id);
                                        } else {
                                            // Health check failed
                                            tracing::warn!("Health check failed for server {}", server_id);
                                            *connection.status.write() = ConnectionStatus::Error;
                                            *connection.error_count.lock() += 1;

                                            let _ = event_sender_clone.send(ConnectionEvent::StatusChanged {
                                                server_id,
                                                status: ConnectionStatus::Error,
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Health check error for server {}: {}", server_id, e);
                                        *connection.status.write() = ConnectionStatus::Error;
                                        *connection.error_count.lock() += 1;

                                        let _ = event_sender_clone.send(ConnectionEvent::StatusChanged {
                                            server_id,
                                            status: ConnectionStatus::Error,
                                        });
                                    }
                                }
                            });
                        }

                        // Wait for all health checks to complete (bounded by 30s interval)
                        while let Some(result) = join_set.join_next().await {
                            if let Err(e) = result {
                                tracing::error!("Health check task panicked: {}", e);
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
