//! Studio-specific elicitation handler that forwards requests to the frontend

use crate::database::Database;
use crate::mcp_client::{ServerContext, CURRENT_SERVER_CONTEXT};
use crate::types::{MessageDirection, MessageHistory};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;
use turbomcp_client::handlers::{
    ElicitationHandler, ElicitationRequest, ElicitationResponse, HandlerError, HandlerResult,
};
use uuid::Uuid;

/// Context-aware elicitation handler wrapper
///
/// This wrapper sets the server context in task-local storage before delegating
/// to the actual elicitation handler. This allows the handler to know which server
/// initiated the request in multi-server scenarios.
#[derive(Debug, Clone)]
pub struct ContextAwareElicitationHandler {
    context: Arc<ServerContext>,
    inner: Arc<StudioElicitationHandler>,
}

impl ContextAwareElicitationHandler {
    pub fn new(context: ServerContext, inner: Arc<StudioElicitationHandler>) -> Self {
        Self {
            context: Arc::new(context),
            inner,
        }
    }
}

#[async_trait::async_trait]
impl ElicitationHandler for ContextAwareElicitationHandler {
    async fn handle_elicitation(
        &self,
        request: ElicitationRequest,
    ) -> HandlerResult<ElicitationResponse> {
        // Set the server context in task-local storage before delegating
        let context = self.context.clone();
        let inner = self.inner.clone();

        CURRENT_SERVER_CONTEXT
            .scope(
                context,
                async move { inner.handle_elicitation(request).await },
            )
            .await
    }
}

/// Studio-specific elicitation handler that forwards requests to the frontend
#[derive(Debug, Clone)]
pub struct StudioElicitationHandler {
    app_handle: tauri::AppHandle,
    pending_requests: Arc<DashMap<String, ElicitationRequest>>,
    response_channels: Arc<DashMap<String, tokio::sync::oneshot::Sender<ElicitationResponse>>>,

    /// Database for protocol message logging (initialized async)
    db: Arc<tokio::sync::RwLock<Option<Arc<Database>>>>,
}

impl StudioElicitationHandler {
    pub fn new(
        app_handle: tauri::AppHandle,
        db: Arc<tokio::sync::RwLock<Option<Arc<Database>>>>,
    ) -> Self {
        tracing::info!("Initializing StudioElicitationHandler with protocol logging");

        Self {
            app_handle,
            pending_requests: Arc::new(DashMap::new()),
            response_channels: Arc::new(DashMap::new()),
            db,
        }
    }

    /// Get server context from task-local storage
    fn get_server_context(&self) -> ServerContext {
        CURRENT_SERVER_CONTEXT
            .try_with(|ctx| (**ctx).clone())
            .unwrap_or_else(|_| {
                tracing::error!("⚠️ No server context in task-local storage for elicitation!");
                ServerContext::default()
            })
    }

    pub fn submit_response(
        &self,
        request_id: String,
        response: ElicitationResponse,
    ) -> Result<(), String> {
        tracing::info!("Submitting elicitation response for: {}", request_id);
        self.pending_requests.remove(&request_id);

        if let Some((_, tx)) = self.response_channels.remove(&request_id) {
            tx.send(response)
                .map_err(|_| "Failed to send response through channel")?;
            Ok(())
        } else {
            Err(format!("No pending channel for request: {}", request_id))
        }
    }
}

#[async_trait::async_trait]
impl ElicitationHandler for StudioElicitationHandler {
    async fn handle_elicitation(
        &self,
        request: ElicitationRequest,
    ) -> HandlerResult<ElicitationResponse> {
        let start = Instant::now();

        tracing::info!(
            "Received elicitation request: {:?} - {}",
            request.id(),
            request.message()
        );

        // Get server context
        let server_context = self.get_server_context();

        // Convert request ID to string early for use throughout
        let request_id_str = request.id().to_string();

        // Capture incoming request for protocol inspector
        let protocol_msg_id = Uuid::new_v4();
        if let Some(db) = self.db.read().await.as_ref() {
            // Serialize a summary of the request for logging
            let request_summary = serde_json::json!({
                "id": &request_id_str,
                "message": request.message(),
                "timeout_ms": request.timeout().map(|d| d.as_millis()),
                "cancellable": request.is_cancellable(),
            });
            let request_json = serde_json::to_string(&request_summary)
                .unwrap_or_else(|_| "Failed to serialize request".to_string());
            let size_bytes = request_json.len() as i64;

            let message = MessageHistory {
                id: protocol_msg_id,
                server_id: server_context.server_id,
                timestamp: Utc::now(),
                direction: MessageDirection::ServerToClient,
                content: request_json,
                size_bytes,
                processing_time_ms: None,
            };

            if let Err(e) = db.save_message(&message).await {
                tracing::warn!("Failed to save incoming elicitation request: {}", e);
            } else {
                let _ = self.app_handle.emit("protocol_message", &protocol_msg_id);
                tracing::debug!(
                    "📝 Captured incoming elicitation request: {}",
                    protocol_msg_id
                );
            }
        }

        // Store request with ID as string key (request_id_str defined earlier)
        self.pending_requests
            .insert(request_id_str.clone(), request.clone());

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.response_channels.insert(request_id_str.clone(), tx);

        // Serialize schema for event (typed schema → Value for JSON)
        let schema_value =
            serde_json::to_value(request.schema()).unwrap_or_else(|_| serde_json::Value::Null);

        // Convert timeout to seconds for display
        let timeout_secs = request.timeout().map(|d| d.as_secs());

        let event_payload = serde_json::json!({
            "id": request_id_str,
            "protocolMessageId": protocol_msg_id.to_string(),  // For Protocol Inspector correlation
            "serverId": server_context.server_id.to_string(),  // Server that initiated the request
            "serverName": server_context.server_name,  // Human-readable server name
            "message": request.message(),
            "requestedSchema": schema_value,
            "timeout": timeout_secs,
            // Removed metadata - not in protocol
        });

        self.app_handle
            .emit("elicitation_requested", event_payload)
            .map_err(|e| HandlerError::Generic {
                message: format!("Failed to emit event: {}", e),
            })?;

        tracing::info!(
            "Emitted elicitation_requested event for: {}",
            request_id_str
        );

        // Use timeout from request (Duration) or default to 5 minutes
        let timeout_duration = request.timeout().unwrap_or(Duration::from_secs(300));

        let response = tokio::time::timeout(timeout_duration, rx)
            .await
            .map_err(|_| {
                self.pending_requests.remove(&request_id_str);
                self.response_channels.remove(&request_id_str);
                HandlerError::Timeout {
                    timeout_seconds: timeout_duration.as_secs(),
                }
            })?
            .map_err(|_| HandlerError::Generic {
                message: "Channel closed before receiving response".to_string(),
            })?;

        tracing::info!(
            "Received elicitation response for: {} - action: {:?}",
            request_id_str,
            response.action()
        );

        // Capture outgoing response for protocol inspector
        let processing_time_ms = start.elapsed().as_millis() as i64;
        if let Some(db) = self.db.read().await.as_ref() {
            let msg_id = Uuid::new_v4();
            // Serialize the action/content for logging (not the full protocol type)
            let response_summary = serde_json::json!({
                "action": match response.action() {
                    turbomcp_client::handlers::ElicitationAction::Accept => "accept",
                    turbomcp_client::handlers::ElicitationAction::Decline => "decline",
                    turbomcp_client::handlers::ElicitationAction::Cancel => "cancel",
                },
                "content": response.content(),
            });
            let response_json = serde_json::to_string(&response_summary)
                .unwrap_or_else(|_| "Failed to serialize response".to_string());
            let size_bytes = response_json.len() as i64;

            let message = MessageHistory {
                id: msg_id,
                server_id: server_context.server_id,
                timestamp: Utc::now(),
                direction: MessageDirection::ClientToServer,
                content: response_json,
                size_bytes,
                processing_time_ms: Some(processing_time_ms),
            };

            if let Err(e) = db.save_message(&message).await {
                tracing::warn!("Failed to save outgoing elicitation response: {}", e);
            } else {
                let _ = self.app_handle.emit("protocol_message", &msg_id);
                tracing::debug!(
                    "📝 Captured outgoing elicitation response: {} ({}ms)",
                    msg_id,
                    processing_time_ms
                );
            }
        }

        Ok(response)
    }
}
