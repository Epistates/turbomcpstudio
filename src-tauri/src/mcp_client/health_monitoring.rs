//! Health Monitoring Module
//!
//! Provides comprehensive health and metrics monitoring for MCP connections:
//! - Per-connection metrics (requests, errors, uptime, response times)
//! - Aggregated metrics across all connections
//! - Real-time metric updates
//!
//! All methods are read-only and safe to call concurrently.

use crate::error::{McpResult, McpStudioError};
use crate::mcp_client::connection::ManagedConnection;
use crate::types::ConnectionMetrics;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Health Monitoring Operations
///
/// Provides stateless operations for connection health and metrics monitoring.
/// All methods require a connection reference from the manager.
pub struct HealthMonitoring;

impl HealthMonitoring {
    /// Get connection metrics for a specific server
    ///
    /// Returns real-time metrics including:
    /// - Request/response counts
    /// - Error counts
    /// - Uptime in seconds
    /// - Average response time
    /// - Bytes sent/received
    pub async fn get_connection_metrics(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
        server_id: Uuid,
    ) -> McpResult<ConnectionMetrics> {
        let connection = connections
            .get(&server_id)
            .ok_or_else(|| McpStudioError::ServerNotFound(server_id.to_string()))?;

        let mut metrics = connection.metrics.read().clone();

        // Update real-time metrics
        metrics.requests_sent = *connection.request_count.lock();
        metrics.error_count = *connection.error_count.lock();

        // Calculate uptime
        if let Some(connected_at) = *connection.connected_at.read() {
            metrics.uptime_seconds = connected_at.elapsed().as_secs();
        }

        Ok(metrics)
    }

    /// Get aggregated metrics across all connections
    ///
    /// Aggregates metrics from all active connections:
    /// - Sums: requests, responses, bytes, errors
    /// - Averages: response time (weighted by request count)
    /// - Max: uptime (longest running connection)
    pub async fn get_aggregated_metrics(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
    ) -> McpResult<ConnectionMetrics> {
        let mut aggregated = ConnectionMetrics::default();

        for entry in connections.iter() {
            let connection = entry.value();
            let metrics = connection.metrics.read();

            aggregated.requests_sent += metrics.requests_sent;
            aggregated.responses_received += metrics.responses_received;
            aggregated.bytes_sent += metrics.bytes_sent;
            aggregated.bytes_received += metrics.bytes_received;
            aggregated.error_count += metrics.error_count;

            // Calculate average response time (weighted by message count)
            if aggregated.requests_sent > 0 {
                let total_time = (aggregated.avg_response_time_ms
                    * (aggregated.requests_sent - metrics.requests_sent) as f64)
                    + (metrics.avg_response_time_ms * metrics.requests_sent as f64);
                aggregated.avg_response_time_ms = total_time / aggregated.requests_sent as f64;
            }

            // Use the maximum uptime as the overall uptime
            aggregated.uptime_seconds = aggregated.uptime_seconds.max(metrics.uptime_seconds);
        }

        Ok(aggregated)
    }

    /// Get all connection metrics as a map
    ///
    /// Returns a HashMap mapping server_id → ConnectionMetrics for all servers.
    /// Useful for dashboard displays and bulk monitoring.
    pub async fn get_all_connection_metrics(
        connections: &DashMap<Uuid, Arc<ManagedConnection>>,
    ) -> HashMap<Uuid, ConnectionMetrics> {
        let mut metrics_map = HashMap::new();
        for entry in connections.iter() {
            let server_id = *entry.key();
            let connection = entry.value();
            let metrics = connection.metrics.read().clone();
            metrics_map.insert(server_id, metrics);
        }
        metrics_map
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_health_monitoring_module_exists() {
        // Smoke test - module compiles
    }

    // TODO(testing): Add integration tests with mock connections
    // - Test metrics aggregation with multiple connections
    // - Test uptime calculation
    // - Test weighted average response time
    // - Test empty connections case
    // - Test concurrent metric reads
}
