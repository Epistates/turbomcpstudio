//! Studio-specific elicitation handler that forwards requests to the frontend

use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use turbomcp_client::handlers::{
    ElicitationHandler, ElicitationRequest, ElicitationResponse, HandlerError, HandlerResult,
};

/// Studio-specific elicitation handler that forwards requests to the frontend
#[derive(Debug, Clone)]
pub struct StudioElicitationHandler {
    app_handle: tauri::AppHandle,
    pending_requests: Arc<DashMap<String, ElicitationRequest>>,
    response_channels: Arc<DashMap<String, tokio::sync::oneshot::Sender<ElicitationResponse>>>,
}

impl StudioElicitationHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            pending_requests: Arc::new(DashMap::new()),
            response_channels: Arc::new(DashMap::new()),
        }
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
        tracing::info!(
            "Received elicitation request: {} - {}",
            request.id,
            request.prompt
        );

        self.pending_requests
            .insert(request.id.clone(), request.clone());

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.response_channels.insert(request.id.clone(), tx);

        let event_payload = serde_json::json!({
            "id": request.id,
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
        Ok(response)
    }
}
