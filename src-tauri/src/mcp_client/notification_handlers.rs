//! Notification Handlers Module
//!
//! Implements MCP server notification handlers:
//! - **LogHandler**: Receives server log messages and forwards to frontend
//! - **ResourceUpdateHandler**: Receives resource change notifications and emits events
//!
//! Both handlers use context-aware wrappers for proper server attribution
//! in multi-server scenarios via task-local storage.

use crate::mcp_client::{ServerContext, CURRENT_SERVER_CONTEXT};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tauri::Emitter;
use turbomcp_client::handlers::{
    HandlerResult, LogHandler, ProgressHandler, PromptListChangedHandler,
    ResourceListChangedHandler, ResourceUpdateHandler, ToolListChangedHandler,
};
use turbomcp_protocol::types::{
    LoggingNotification, ProgressNotification, ResourceUpdatedNotification,
};

// ============================================================================
// Log Handler
// ============================================================================

/// Studio log handler that forwards server log messages to the frontend
///
/// Receives `notifications/message` from MCP servers and emits Tauri events
/// so the Protocol Inspector and log views can display them in real-time.
#[derive(Debug, Clone)]
pub struct StudioLogHandler {
    app_handle: tauri::AppHandle,
}

impl StudioLogHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl LogHandler for StudioLogHandler {
    fn handle_log(
        &self,
        log: LoggingNotification,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let app_handle = self.app_handle.clone();

        Box::pin(async move {
            let server_context = CURRENT_SERVER_CONTEXT
                .try_with(|ctx| (**ctx).clone())
                .unwrap_or_default();

            let level_str = format!("{:?}", log.level);

            tracing::debug!(
                "Server log from {} [{}]: {}",
                server_context.server_name,
                level_str,
                log.data
            );

            let payload = serde_json::json!({
                "serverId": server_context.server_id.to_string(),
                "serverName": server_context.server_name,
                "level": level_str,
                "data": log.data,
                "logger": log.logger,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = app_handle.emit("server_log_message", &payload) {
                tracing::warn!("Failed to emit server log event: {}", e);
            }

            Ok(())
        })
    }
}

/// Context-aware log handler wrapper
///
/// Sets the server context in task-local storage before delegating
/// to the actual log handler for proper server attribution.
#[derive(Debug, Clone)]
pub struct ContextAwareLogHandler {
    context: Arc<ServerContext>,
    inner: Arc<StudioLogHandler>,
}

impl ContextAwareLogHandler {
    pub fn new(context: ServerContext, inner: Arc<StudioLogHandler>) -> Self {
        Self {
            context: Arc::new(context),
            inner,
        }
    }
}

impl LogHandler for ContextAwareLogHandler {
    fn handle_log(
        &self,
        log: LoggingNotification,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let context = self.context.clone();
        let inner = self.inner.clone();

        Box::pin(async move {
            CURRENT_SERVER_CONTEXT
                .scope(context, async move { inner.handle_log(log).await })
                .await
        })
    }
}

// ============================================================================
// Resource Update Handler
// ============================================================================

/// Studio resource update handler that notifies frontend of resource changes
///
/// Receives `notifications/resources/updated` from MCP servers when a
/// subscribed resource has changed. Emits Tauri events so the frontend
/// can refresh the affected resource.
#[derive(Debug, Clone)]
pub struct StudioResourceUpdateHandler {
    app_handle: tauri::AppHandle,
}

impl StudioResourceUpdateHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl ResourceUpdateHandler for StudioResourceUpdateHandler {
    fn handle_resource_update(
        &self,
        notification: ResourceUpdatedNotification,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let app_handle = self.app_handle.clone();

        Box::pin(async move {
            let server_context = CURRENT_SERVER_CONTEXT
                .try_with(|ctx| (**ctx).clone())
                .unwrap_or_default();

            let uri_str = notification.uri.to_string();

            tracing::info!(
                "Resource updated on server {}: {}",
                server_context.server_name,
                uri_str
            );

            let payload = serde_json::json!({
                "serverId": server_context.server_id.to_string(),
                "serverName": server_context.server_name,
                "uri": uri_str,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = app_handle.emit("resource_updated", &payload) {
                tracing::warn!("Failed to emit resource_updated event: {}", e);
            }

            Ok(())
        })
    }
}

/// Context-aware resource update handler wrapper
#[derive(Debug, Clone)]
pub struct ContextAwareResourceUpdateHandler {
    context: Arc<ServerContext>,
    inner: Arc<StudioResourceUpdateHandler>,
}

impl ContextAwareResourceUpdateHandler {
    pub fn new(context: ServerContext, inner: Arc<StudioResourceUpdateHandler>) -> Self {
        Self {
            context: Arc::new(context),
            inner,
        }
    }
}

impl ResourceUpdateHandler for ContextAwareResourceUpdateHandler {
    fn handle_resource_update(
        &self,
        notification: ResourceUpdatedNotification,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let context = self.context.clone();
        let inner = self.inner.clone();

        Box::pin(async move {
            CURRENT_SERVER_CONTEXT
                .scope(context, async move {
                    inner.handle_resource_update(notification).await
                })
                .await
        })
    }
}

// ============================================================================
// Progress Handler
// ============================================================================

/// Studio progress handler that forwards progress notifications to the frontend
///
/// Receives `notifications/progress` from MCP servers for long-running operations
/// and emits Tauri events so the UI can display progress bars/indicators.
#[derive(Debug, Clone)]
pub struct StudioProgressHandler {
    app_handle: tauri::AppHandle,
}

impl StudioProgressHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl ProgressHandler for StudioProgressHandler {
    fn handle_progress(
        &self,
        notification: ProgressNotification,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let app_handle = self.app_handle.clone();

        Box::pin(async move {
            let server_context = CURRENT_SERVER_CONTEXT
                .try_with(|ctx| (**ctx).clone())
                .unwrap_or_default();

            tracing::debug!(
                "Progress from {} [{}]: {}/{}",
                server_context.server_name,
                notification.progress_token,
                notification.progress,
                notification.total.map_or("?".to_string(), |t| t.to_string()),
            );

            let payload = serde_json::json!({
                "serverId": server_context.server_id.to_string(),
                "serverName": server_context.server_name,
                "progressToken": notification.progress_token,
                "progress": notification.progress,
                "total": notification.total,
                "message": notification.message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = app_handle.emit("server_progress", &payload) {
                tracing::warn!("Failed to emit server_progress event: {}", e);
            }

            Ok(())
        })
    }
}

/// Context-aware progress handler wrapper
#[derive(Debug, Clone)]
pub struct ContextAwareProgressHandler {
    context: Arc<ServerContext>,
    inner: Arc<StudioProgressHandler>,
}

impl ContextAwareProgressHandler {
    pub fn new(context: ServerContext, inner: Arc<StudioProgressHandler>) -> Self {
        Self {
            context: Arc::new(context),
            inner,
        }
    }
}

impl ProgressHandler for ContextAwareProgressHandler {
    fn handle_progress(
        &self,
        notification: ProgressNotification,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let context = self.context.clone();
        let inner = self.inner.clone();

        Box::pin(async move {
            CURRENT_SERVER_CONTEXT
                .scope(context, async move {
                    inner.handle_progress(notification).await
                })
                .await
        })
    }
}

// ============================================================================
// List Changed Handlers
// ============================================================================

/// Studio tool list changed handler
///
/// Receives `notifications/tools/list_changed` and emits a Tauri event
/// so the frontend can refresh the tool list for the affected server.
#[derive(Debug, Clone)]
pub struct StudioToolListChangedHandler {
    app_handle: tauri::AppHandle,
}

impl StudioToolListChangedHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl ToolListChangedHandler for StudioToolListChangedHandler {
    fn handle_tool_list_changed(
        &self,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let app_handle = self.app_handle.clone();

        Box::pin(async move {
            let server_context = CURRENT_SERVER_CONTEXT
                .try_with(|ctx| (**ctx).clone())
                .unwrap_or_default();

            tracing::info!(
                "Tool list changed on server: {}",
                server_context.server_name
            );

            let payload = serde_json::json!({
                "serverId": server_context.server_id.to_string(),
                "serverName": server_context.server_name,
                "listType": "tools",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = app_handle.emit("list_changed", &payload) {
                tracing::warn!("Failed to emit list_changed event: {}", e);
            }

            Ok(())
        })
    }
}

/// Context-aware tool list changed handler wrapper
#[derive(Debug, Clone)]
pub struct ContextAwareToolListChangedHandler {
    context: Arc<ServerContext>,
    inner: Arc<StudioToolListChangedHandler>,
}

impl ContextAwareToolListChangedHandler {
    pub fn new(context: ServerContext, inner: Arc<StudioToolListChangedHandler>) -> Self {
        Self {
            context: Arc::new(context),
            inner,
        }
    }
}

impl ToolListChangedHandler for ContextAwareToolListChangedHandler {
    fn handle_tool_list_changed(
        &self,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let context = self.context.clone();
        let inner = self.inner.clone();

        Box::pin(async move {
            CURRENT_SERVER_CONTEXT
                .scope(context, async move {
                    inner.handle_tool_list_changed().await
                })
                .await
        })
    }
}

/// Studio prompt list changed handler
#[derive(Debug, Clone)]
pub struct StudioPromptListChangedHandler {
    app_handle: tauri::AppHandle,
}

impl StudioPromptListChangedHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl PromptListChangedHandler for StudioPromptListChangedHandler {
    fn handle_prompt_list_changed(
        &self,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let app_handle = self.app_handle.clone();

        Box::pin(async move {
            let server_context = CURRENT_SERVER_CONTEXT
                .try_with(|ctx| (**ctx).clone())
                .unwrap_or_default();

            tracing::info!(
                "Prompt list changed on server: {}",
                server_context.server_name
            );

            let payload = serde_json::json!({
                "serverId": server_context.server_id.to_string(),
                "serverName": server_context.server_name,
                "listType": "prompts",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = app_handle.emit("list_changed", &payload) {
                tracing::warn!("Failed to emit list_changed event: {}", e);
            }

            Ok(())
        })
    }
}

/// Context-aware prompt list changed handler wrapper
#[derive(Debug, Clone)]
pub struct ContextAwarePromptListChangedHandler {
    context: Arc<ServerContext>,
    inner: Arc<StudioPromptListChangedHandler>,
}

impl ContextAwarePromptListChangedHandler {
    pub fn new(context: ServerContext, inner: Arc<StudioPromptListChangedHandler>) -> Self {
        Self {
            context: Arc::new(context),
            inner,
        }
    }
}

impl PromptListChangedHandler for ContextAwarePromptListChangedHandler {
    fn handle_prompt_list_changed(
        &self,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let context = self.context.clone();
        let inner = self.inner.clone();

        Box::pin(async move {
            CURRENT_SERVER_CONTEXT
                .scope(context, async move {
                    inner.handle_prompt_list_changed().await
                })
                .await
        })
    }
}

/// Studio resource list changed handler
#[derive(Debug, Clone)]
pub struct StudioResourceListChangedHandler {
    app_handle: tauri::AppHandle,
}

impl StudioResourceListChangedHandler {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl ResourceListChangedHandler for StudioResourceListChangedHandler {
    fn handle_resource_list_changed(
        &self,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let app_handle = self.app_handle.clone();

        Box::pin(async move {
            let server_context = CURRENT_SERVER_CONTEXT
                .try_with(|ctx| (**ctx).clone())
                .unwrap_or_default();

            tracing::info!(
                "Resource list changed on server: {}",
                server_context.server_name
            );

            let payload = serde_json::json!({
                "serverId": server_context.server_id.to_string(),
                "serverName": server_context.server_name,
                "listType": "resources",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = app_handle.emit("list_changed", &payload) {
                tracing::warn!("Failed to emit list_changed event: {}", e);
            }

            Ok(())
        })
    }
}

/// Context-aware resource list changed handler wrapper
#[derive(Debug, Clone)]
pub struct ContextAwareResourceListChangedHandler {
    context: Arc<ServerContext>,
    inner: Arc<StudioResourceListChangedHandler>,
}

impl ContextAwareResourceListChangedHandler {
    pub fn new(context: ServerContext, inner: Arc<StudioResourceListChangedHandler>) -> Self {
        Self {
            context: Arc::new(context),
            inner,
        }
    }
}

impl ResourceListChangedHandler for ContextAwareResourceListChangedHandler {
    fn handle_resource_list_changed(
        &self,
    ) -> Pin<Box<dyn Future<Output = HandlerResult<()>> + Send + '_>> {
        let context = self.context.clone();
        let inner = self.inner.clone();

        Box::pin(async move {
            CURRENT_SERVER_CONTEXT
                .scope(context, async move {
                    inner.handle_resource_list_changed().await
                })
                .await
        })
    }
}
