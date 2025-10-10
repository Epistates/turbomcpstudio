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
            "Received elicitation request: {} - {}",
            request.id,
            request.prompt
        );

        // Get server context
        let server_context = self.get_server_context();

        // Capture incoming request for protocol inspector
        let protocol_msg_id = Uuid::new_v4();
        if let Some(db) = self.db.read().await.as_ref() {
            let request_json = serde_json::to_string(&request)
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

        self.pending_requests
            .insert(request.id.clone(), request.clone());

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.response_channels.insert(request.id.clone(), tx);

        let event_payload = serde_json::json!({
            "id": request.id,
            "protocolMessageId": protocol_msg_id.to_string(),  // For Protocol Inspector correlation
            "message": request.prompt,
            "requestedSchema": request.schema,
            "timeout": request.timeout,
            "metadata": request.metadata,
        });

        self.app_handle
            .emit("elicitation_requested", event_payload)
            .map_err(|e| HandlerError::Generic {
                message: format!("Failed to emit event: {}", e),
            })?;

        tracing::info!("Emitted elicitation_requested event for: {}", request.id);

        let response =
            tokio::time::timeout(Duration::from_secs(request.timeout.unwrap_or(300)), rx)
                .await
                .map_err(|_| {
                    self.pending_requests.remove(&request.id);
                    self.response_channels.remove(&request.id);
                    HandlerError::Timeout {
                        timeout_seconds: request.timeout.unwrap_or(300),
                    }
                })?
                .map_err(|_| HandlerError::Generic {
                    message: "Channel closed before receiving response".to_string(),
                })?;

        tracing::info!(
            "Received elicitation response for: {} - action: {:?}",
            request.id,
            response.action
        );

        // Capture outgoing response for protocol inspector
        let processing_time_ms = start.elapsed().as_millis() as i64;
        if let Some(db) = self.db.read().await.as_ref() {
            let msg_id = Uuid::new_v4();
            let response_json = serde_json::to_string(&response)
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
