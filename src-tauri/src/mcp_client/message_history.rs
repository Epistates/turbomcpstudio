//! Message History Module
//!
//! Provides Protocol Inspector functionality for MCP Studio:
//! - Captures all MCP protocol traffic (requests and responses)
//! - Tracks message timing and latency
//! - Persists to SQLite database for debugging and replay
//! - Enables protocol analysis and performance monitoring

use crate::error::{McpResult, McpStudioError};
use std::sync::Arc;
use uuid::Uuid;

/// Message History Operations
///
/// Provides stateless operations for tracking and persisting MCP protocol messages.
pub struct MessageHistory;

impl MessageHistory {
    /// Track request and save to message history (Protocol Inspector)
    ///
    /// Returns (start_time, unit) for timing tracking
    pub async fn track_request_with_history(
        server_id: Uuid,
        method: &str,
        request_data: &serde_json::Value,
        database: &Arc<crate::database::Database>,
    ) -> McpResult<(u64, ())> {
        let start_time = chrono::Utc::now().timestamp_millis() as u64;

        // Save request message to history
        Self::save_message_to_history(
            server_id,
            request_data.clone(),
            turbomcp_transport::MessageDirection::ClientToServer,
            None,
            database,
        )
        .await?;

        tracing::trace!("Tracked request to history: method={}", method);

        Ok((start_time, ()))
    }

    /// Track request start timing (stub implementation)
    ///
    /// TODO: Implement comprehensive request timing tracking
    pub async fn track_request_start(
        _request_id: Uuid,
        _method: &str,
        _request_size: u64,
    ) -> McpResult<(u64, ())> {
        // TODO: Implement request timing tracking
        let start_time = chrono::Utc::now().timestamp_millis() as u64;
        Ok((start_time, ()))
    }

    /// Track response and save to message history with latency (Protocol Inspector)
    ///
    /// Calculates processing time from start_time and persists with message.
    pub async fn track_response_with_history(
        server_id: Uuid,
        start_time: u64,
        response_data: &serde_json::Value,
        database: &Arc<crate::database::Database>,
    ) -> McpResult<()> {
        let end_time = chrono::Utc::now().timestamp_millis() as u64;
        let processing_time_ms = (end_time - start_time) as i64;

        // Create response message with latency tracking
        let content = serde_json::to_string_pretty(&response_data)
            .map_err(McpStudioError::SerializationError)?;

        let message = crate::types::MessageHistory {
            id: Uuid::new_v4(),
            server_id,
            timestamp: chrono::Utc::now(),
            direction: turbomcp_transport::MessageDirection::ServerToClient,
            content,
            size_bytes: response_data.to_string().len() as i64,
            processing_time_ms: Some(processing_time_ms), // Track latency!
        };

        // Save to database
        database.save_message(&message).await?;

        tracing::trace!(
            "Tracked response to history: latency={}ms",
            processing_time_ms
        );

        Ok(())
    }

    /// Track request error (stub implementation)
    ///
    /// TODO: Implement comprehensive error tracking
    pub async fn track_request_error(_request_id: Uuid, _error: &str) -> McpResult<()> {
        // TODO: Implement error tracking
        Ok(())
    }

    /// Save a message to history database (Protocol Inspector feature)
    ///
    /// Captures all MCP protocol traffic for debugging and replay.
    /// Used by both request and response tracking.
    pub async fn save_message_to_history(
        server_id: Uuid,
        message_data: serde_json::Value,
        direction: turbomcp_transport::MessageDirection,
        timestamp: Option<chrono::DateTime<chrono::Utc>>,
        database: &Arc<crate::database::Database>,
    ) -> McpResult<()> {
        // Serialize message to JSON string
        let content = serde_json::to_string_pretty(&message_data)
            .map_err(McpStudioError::SerializationError)?;

        let size_bytes = content.len() as i64;

        // Create message history record
        let message = crate::types::MessageHistory {
            id: Uuid::new_v4(),
            server_id,
            timestamp: timestamp.unwrap_or_else(chrono::Utc::now),
            direction,
            content,
            size_bytes,
            processing_time_ms: None, // Will be updated for responses with timing
        };

        // Save to database
        database.save_message(&message).await?;

        tracing::debug!(
            "Saved message to history: server={}, direction={:?}, size={}",
            server_id,
            direction,
            size_bytes
        );

        Ok(())
    }
}
