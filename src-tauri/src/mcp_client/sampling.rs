//! Studio-specific sampling handler with human-in-the-loop approval
//!
//! This module implements the production MCP sampling flow:
//! 1. Server requests sampling via `sampling/createMessage`
//! 2. Studio intercepts and presents to user for approval
//! 3. User reviews/modifies request
//! 4. Studio forwards to configured LLM provider
//! 5. User reviews response
//! 6. Studio returns response to server
//!
//! ## Architecture
//!
//! ```text
//! MCP Server → sampling/createMessage → StudioSamplingHandler
//!                                           ↓
//!                                   Emit Tauri event
//!                                           ↓
//!                                   Frontend Modal (HITL)
//!                                           ↓
//!                                   User approves
//!                                           ↓
//!                                   Call LLM via LLMConfigManager
//!                                           ↓
//!                                   User reviews response
//!                                           ↓
//!                                   Return to server
//! ```

use crate::database::Database;
use crate::llm_config::LLMConfigManager;
use crate::mcp_client::{ServerContext, CURRENT_SERVER_CONTEXT};
use crate::types::{MessageDirection, MessageHistory};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;
use turbomcp_client::handlers::HandlerError;
use turbomcp_client::sampling::SamplingHandler;
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult};
use uuid::Uuid;

/// Context-aware sampling handler wrapper
///
/// This wrapper sets the server context in task-local storage before delegating
/// to the actual sampling handler. This allows the handler to know which server
/// initiated the request in multi-server scenarios.
#[derive(Debug, Clone)]
pub struct ContextAwareSamplingHandler {
    context: Arc<ServerContext>,
    inner: Arc<StudioSamplingHandler>,
}

impl ContextAwareSamplingHandler {
    pub fn new(context: ServerContext, inner: Arc<StudioSamplingHandler>) -> Self {
        Self {
            context: Arc::new(context),
            inner,
        }
    }
}

#[async_trait]
impl SamplingHandler for ContextAwareSamplingHandler {
    async fn handle_create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<CreateMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        // Set the server context in task-local storage before delegating
        let context = self.context.clone();
        let inner = self.inner.clone();

        CURRENT_SERVER_CONTEXT
            .scope(context, async move {
                inner.handle_create_message(request).await
            })
            .await
    }
}

/// Pending sampling request awaiting user approval
#[derive(Debug, Clone, Serialize)]
pub struct PendingSamplingRequest {
    pub request_id: String,
    pub server_id: String,
    pub server_name: String,
    pub request: CreateMessageRequest,
    pub estimated_cost: Option<f64>,
    pub estimated_tokens: Option<u32>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
}

/// Studio sampling handler with human-in-the-loop
///
/// This handler intercepts server-initiated sampling requests and presents them
/// to the user for approval before forwarding to the actual LLM provider.
#[derive(Debug, Clone)]
pub struct StudioSamplingHandler {
    /// Tauri app handle for emitting events
    app_handle: tauri::AppHandle,

    /// Pending requests awaiting approval
    pending_requests: Arc<DashMap<String, PendingSamplingRequest>>,

    /// Response channels for async communication (supports both success and rejection)
    /// Uses Box<dyn Error> for errors to preserve error type information (e.g., HandlerError::UserCancelled)
    response_channels: Arc<DashMap<String, tokio::sync::oneshot::Sender<Result<CreateMessageResult, Box<dyn std::error::Error + Send + Sync>>>>>,

    /// Request hash tracking for retry detection (maps content hash → (request_id, count))
    request_hashes: Arc<DashMap<String, (String, u32)>>,

    /// LLM configuration manager for actual LLM calls
    llm_config: Arc<LLMConfigManager>,

    /// Database for protocol message logging (initialized async)
    db: Arc<tokio::sync::RwLock<Option<Arc<Database>>>>,
}

impl StudioSamplingHandler {
    /// Create a new sampling handler
    pub fn new(
        app_handle: tauri::AppHandle,
        llm_config: Arc<LLMConfigManager>,
        db: Arc<tokio::sync::RwLock<Option<Arc<Database>>>>,
    ) -> Self {
        tracing::info!("Initializing StudioSamplingHandler with protocol logging");

        Self {
            app_handle,
            pending_requests: Arc::new(DashMap::new()),
            response_channels: Arc::new(DashMap::new()),
            request_hashes: Arc::new(DashMap::new()),
            llm_config,
            db,
        }
    }

    /// Get server context from task-local storage
    ///
    /// This reads the server context that was set before the handler was invoked.
    /// Falls back to a default context if none is available (shouldn't happen in practice).
    fn get_server_context(&self) -> ServerContext {
        CURRENT_SERVER_CONTEXT
            .try_with(|ctx| (**ctx).clone())
            .unwrap_or_else(|_| {
                tracing::error!("⚠️ No server context in task-local storage!");
                tracing::error!(
                    "This indicates the manager didn't set context before calling the handler"
                );
                ServerContext::default()
            })
    }

    /// Estimate cost of a sampling request (rough approximation)
    fn estimate_cost(&self, request: &CreateMessageRequest) -> Option<f64> {
        // Rough token estimation (very approximate)
        let input_tokens: usize = request
            .messages
            .iter()
            .map(|m| match &m.content {
                turbomcp_protocol::types::Content::Text(t) => t.text.len() / 4, // ~4 chars per token
                _ => 100, // Rough guess for non-text
            })
            .sum();

        let output_tokens = request.max_tokens as usize; // MCP 2025-06-18: Always required

        // GPT-4 pricing (example - should be per-provider)
        let input_cost = (input_tokens as f64) * 0.00003; // $0.03 per 1K tokens
        let output_cost = (output_tokens as f64) * 0.00006; // $0.06 per 1K tokens

        Some(input_cost + output_cost)
    }

    /// Estimate tokens in request
    fn estimate_tokens(&self, request: &CreateMessageRequest) -> Option<u32> {
        let input_tokens: u32 = request
            .messages
            .iter()
            .map(|m| match &m.content {
                turbomcp_protocol::types::Content::Text(t) => (t.text.len() / 4) as u32,
                _ => 100,
            })
            .sum();

        Some(input_tokens + request.max_tokens) // MCP 2025-06-18: Always required
    }

    /// Submit approved request and call LLM
    ///
    /// Called from frontend when user approves the request.
    /// This spawns a task to call the actual LLM and sends result through channel.
    pub fn submit_approved_request(
        &self,
        request_id: String,
        approved_request: CreateMessageRequest,
    ) -> Result<(), String> {
        tracing::info!("User approved sampling request: {}", request_id);

        // Remove from pending
        self.pending_requests.remove(&request_id);

        // Get response channel
        let tx = self
            .response_channels
            .remove(&request_id)
            .ok_or_else(|| format!("No pending channel for request: {}", request_id))?
            .1;

        // Spawn task to call LLM (async operation)
        let llm_config = self.llm_config.clone();
        tokio::spawn(async move {
            tracing::info!("🚀 Calling LLM for approved request: {}", request_id);

            // Use the new invoke_llm_directly method to bypass the DelegatingSamplingHandler
            // and directly call the configured LLM provider
            match llm_config.invoke_llm_directly(approved_request, None).await {
                Ok(result) => {
                    tracing::info!("✅ LLM call succeeded for request: {}", request_id);
                    if tx.send(Ok(result)).is_err() {
                        tracing::error!(
                            "❌ Failed to send LLM response (channel closed): {}",
                            request_id
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("❌ LLM call failed: {} for request: {}", e, request_id);
                    // Use HandlerError::Generic for LLM failures (retryable)
                    let llm_error = HandlerError::Generic {
                        message: format!("LLM call failed: {}", e)
                    };
                    let boxed_error: Box<dyn std::error::Error + Send + Sync> = Box::new(llm_error);
                    if tx.send(Err(boxed_error)).is_err() {
                        tracing::error!("❌ Failed to send LLM error (channel closed): {}", request_id);
                    }
                }
            }
        });

        Ok(())
    }

    /// Submit manual response (bypass LLM - for testing tool quick responses)
    ///
    /// Called from frontend when user provides a manual response via quick templates.
    /// This directly sends the response without calling the LLM.
    pub fn submit_manual_response(
        &self,
        request_id: String,
        manual_response: CreateMessageResult,
    ) -> Result<(), String> {
        tracing::info!("User provided manual response for: {}", request_id);

        // Remove from pending
        self.pending_requests.remove(&request_id);

        // Get response channel
        let tx = self
            .response_channels
            .remove(&request_id)
            .ok_or_else(|| format!("No pending channel for request: {}", request_id))?
            .1;

        // Send manual response as success through channel
        if tx.send(Ok(manual_response)).is_err() {
            tracing::error!(
                "❌ Failed to send manual response (channel closed): {}",
                request_id
            );
            return Err("Channel closed".to_string());
        }

        tracing::info!("✅ Manual response sent for request: {}", request_id);
        Ok(())
    }

    /// Reject sampling request
    ///
    /// Called from frontend when user declines the request.
    pub fn reject_request(&self, request_id: String, reason: String) -> Result<(), String> {
        tracing::info!(
            "User rejected sampling request: {} - {}",
            request_id,
            reason
        );

        // Remove from pending
        self.pending_requests.remove(&request_id);

        // Get response channel
        let tx = self
            .response_channels
            .remove(&request_id)
            .ok_or_else(|| format!("No pending channel for request: {}", request_id))?
            .1;

        // Send rejection error using HandlerError::UserCancelled
        // Maps to JSON-RPC error -32800 (user action)
        // TurboMCP client will preserve this error code via downcast
        // CRITICAL: This ensures the retry logic recognizes this as a user action and doesn't retry
        let rejection_error = HandlerError::UserCancelled;
        let boxed_error: Box<dyn std::error::Error + Send + Sync> = Box::new(rejection_error);
        if tx.send(Err(boxed_error)).is_err() {
            tracing::error!(
                "❌ Failed to send rejection (channel closed): {}",
                request_id
            );
            return Err("Channel closed".to_string());
        }

        tracing::info!("✅ Rejection sent for request: {} - {}", request_id, reason);
        Ok(())
    }
}

#[async_trait]
impl SamplingHandler for StudioSamplingHandler {
    async fn handle_create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<CreateMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
        let request_id = Uuid::new_v4().to_string();

        tracing::info!(
            "🎯 Received sampling request from server (request_id: {})",
            request_id
        );

        // Get server context (workaround - see comments above)
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
                processing_time_ms: None, // Will be updated when response is captured
            };

            if let Err(e) = db.save_message(&message).await {
                tracing::warn!("Failed to save incoming sampling request: {}", e);
            } else {
                // Emit real-time event for UI
                let _ = self.app_handle.emit("protocol_message", &protocol_msg_id);
                tracing::debug!("📝 Captured incoming sampling request: {}", protocol_msg_id);
            }
        }

        // Detect retries by hashing request content
        let request_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            // Hash key components to detect retries
            server_context.server_id.hash(&mut hasher);
            request.messages.len().hash(&mut hasher);
            if let Some(first_msg) = request.messages.first() {
                // ContentBlock is an enum, extract text from Text variant
                match &first_msg.content {
                    turbomcp_protocol::types::ContentBlock::Text(text_content) => {
                        text_content.text.hash(&mut hasher);
                    },
                    _ => {
                        // For non-text content, hash a placeholder
                        "non-text-content".hash(&mut hasher);
                    }
                }
            }
            format!("{:x}", hasher.finish())
        };

        let retry_count = if let Some(mut entry) = self.request_hashes.get_mut(&request_hash) {
            // We've seen this request before - it's a retry!
            let (prev_id, count) = &mut *entry;
            *count += 1;
            *prev_id = request_id.clone(); // Update to current request ID
            tracing::info!("🔄 Detected retry #{} for request content hash: {}", count, &request_hash[..8]);
            Some(*count)
        } else {
            // First time seeing this request
            self.request_hashes.insert(request_hash.clone(), (request_id.clone(), 1));
            None
        };

        // Create pending request
        let pending = PendingSamplingRequest {
            request_id: request_id.clone(),
            server_id: server_context.server_id.to_string(),
            server_name: server_context.server_name.clone(),
            request: request.clone(),
            estimated_cost: self.estimate_cost(&request),
            estimated_tokens: self.estimate_tokens(&request),
            created_at: chrono::Utc::now().to_rfc3339(),
            retry_count,
        };

        // Store pending request
        self.pending_requests
            .insert(request_id.clone(), pending.clone());

        // Create response channel
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.response_channels.insert(request_id.clone(), tx);

        // Emit event to frontend for HITL approval
        let event_payload = serde_json::json!({
            "requestId": request_id,
            "protocolMessageId": protocol_msg_id.to_string(),  // For Protocol Inspector correlation
            "serverId": server_context.server_id,
            "serverName": server_context.server_name,
            "request": {
                "messages": request.messages,
                "modelPreferences": request.model_preferences,
                "systemPrompt": request.system_prompt,
                "includeContext": request.include_context,
                "temperature": request.temperature,
                "maxTokens": request.max_tokens,
            },
            "estimatedCost": pending.estimated_cost,
            "estimatedTokens": pending.estimated_tokens,
            "createdAt": pending.created_at,
            "retryCount": pending.retry_count,
        });

        self.app_handle
            .emit("sampling_requested", event_payload)
            .map_err(|e| format!("Failed to emit event: {}", e))?;

        tracing::info!("✅ Emitted sampling_requested event: {}", request_id);

        // Wait for user approval + LLM result (with timeout)
        tracing::info!("⏳ Waiting for user approval: {}", request_id);

        let result = tokio::time::timeout(Duration::from_secs(300), rx)
            .await
            .map_err(|_| {
                self.pending_requests.remove(&request_id);
                self.response_channels.remove(&request_id);
                // User didn't respond in time (5 minute timeout)
                // HandlerError::Timeout maps to JSON-RPC -32801
                Box::new(HandlerError::Timeout { timeout_seconds: 300 }) as Box<dyn std::error::Error + Send + Sync>
            })?
            .map_err(|_| {
                // Channel closed before response - internal error
                Box::new(HandlerError::Generic {
                    message: "Channel closed before receiving response".to_string()
                }) as Box<dyn std::error::Error + Send + Sync>
            })?
            // Unwrap inner Result - if Err, this is the user's rejection (HandlerError::UserCancelled)
            ?;

        tracing::info!("🎉 Sampling request completed: {}", request_id);

        // Capture outgoing response for protocol inspector
        let processing_time_ms = start.elapsed().as_millis() as i64;
        if let Some(db) = self.db.read().await.as_ref() {
            let msg_id = Uuid::new_v4();
            let response_json = serde_json::to_string(&result)
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
                tracing::warn!("Failed to save outgoing sampling response: {}", e);
            } else {
                // Emit real-time event for UI
                let _ = self.app_handle.emit("protocol_message", &msg_id);
                tracing::debug!(
                    "📝 Captured outgoing sampling response: {} ({}ms)",
                    msg_id,
                    processing_time_ms
                );
            }
        }

        Ok(result)
    }
}
