//! ProxyManager - Main orchestrator for proxy lifecycle
//!
//! Integrates with turbomcp-proxy for introspection and runtime capabilities.

use super::types::*;
use crate::database::Database;
use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Import turbomcp-proxy introspection types
use turbomcp_proxy::introspection::{McpBackend, McpIntrospector, StdioBackend};
use turbomcp_proxy::introspection::spec::ServerSpec as TurboServerSpec;

/// Main proxy manager for creating, starting, stopping, and monitoring proxies
pub struct ProxyManager {
    database: Arc<RwLock<Option<Arc<Database>>>>,
    running_proxies: Arc<RwLock<HashMap<String, RunningProxyInfo>>>,
}

/// Information about a running proxy
#[derive(Debug, Clone)]
pub struct RunningProxyInfo {
    pub config: ProxyConfig,
    pub started_at: std::time::SystemTime,
    pub metrics: ProxyMetrics,
    pub frontend_url: Option<String>,
}

impl ProxyManager {
    /// Create a new proxy manager
    pub fn new(database: Arc<RwLock<Option<Arc<Database>>>>) -> Self {
        Self {
            database,
            running_proxies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new proxy configuration (doesn't start it)
    pub async fn create_proxy(
        &self,
        name: String,
        description: Option<String>,
        backend_config: BackendConfig,
        frontend_type: FrontendType,
        auth_config: AuthConfig,
    ) -> AppResult<ProxyId> {
        let proxy_id = ProxyId::new();
        let now = std::time::SystemTime::now();

        let config = ProxyConfig {
            id: proxy_id.clone(),
            name,
            description,
            backend_type: match &backend_config {
                BackendConfig::Stdio { .. } => "stdio",
                BackendConfig::Http { .. } => "http",
                BackendConfig::Tcp { .. } => "tcp",
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                BackendConfig::Unix { .. } => "unix",
                BackendConfig::WebSocket { .. } => "websocket",
            }
            .to_string(),
            backend_config: serde_json::to_value(&backend_config)?,
            frontend_type,
            frontend_config: serde_json::json!({ "enabled": false }),
            auth_config,
            metrics_enabled: true,
            max_requests_tracked: 10000,
            created_at: now,
            updated_at: now,
            last_started_at: None,
            last_stopped_at: None,
        };

        // Persist to database
        let db = self
            .database
            .read()
            .await
            .as_ref()
            .ok_or_else(|| AppError::proxy("Database not initialized"))?
            .clone();

        db.save_proxy_config(&config).await?;

        Ok(proxy_id)
    }

    /// Start a proxy (initialize backend, bind frontend)
    pub async fn start_proxy(&self, proxy_id: &ProxyId) -> AppResult<ProxyStatus> {
        let db = self
            .database
            .read()
            .await
            .as_ref()
            .ok_or_else(|| AppError::proxy("Database not initialized"))?
            .clone();

        let config = db
            .get_proxy_config(&proxy_id.0)
            .await?
            .ok_or_else(|| AppError::proxy(format!("Proxy {} not found", proxy_id)))?;

        // For now, just mark as running and return status
        // Full implementation will integrate turbomcp-proxy
        let running_info = RunningProxyInfo {
            config: config.clone(),
            started_at: std::time::SystemTime::now(),
            metrics: ProxyMetrics::default(),
            frontend_url: Some(format!("http://localhost:3000/proxy/{}", proxy_id.0)),
        };

        let mut proxies = self.running_proxies.write().await;
        proxies.insert(proxy_id.0.clone(), running_info.clone());

        // Update database
        let mut updated_config = config;
        updated_config.last_started_at = Some(std::time::SystemTime::now());
        db.save_proxy_config(&updated_config).await?;

        Ok(ProxyStatus {
            id: proxy_id.clone(),
            name: updated_config.name,
            running: true,
            frontend_url: Some(format!("http://localhost:3000/proxy/{}", proxy_id.0)),
            uptime_seconds: Some(0),
            total_requests: Some(0),
            error_count: Some(0),
            avg_latency_ms: Some(0.0),
            last_error: None,
        })
    }

    /// Stop a proxy (gracefully shutdown)
    pub async fn stop_proxy(&self, proxy_id: &ProxyId) -> AppResult<()> {
        let db = self
            .database
            .read()
            .await
            .as_ref()
            .ok_or_else(|| AppError::proxy("Database not initialized"))?
            .clone();

        let config = db
            .get_proxy_config(&proxy_id.0)
            .await?
            .ok_or_else(|| AppError::proxy(format!("Proxy {} not found", proxy_id)))?;

        let mut proxies = self.running_proxies.write().await;
        proxies.remove(&proxy_id.0);

        // Update database
        let mut updated_config = config;
        updated_config.last_stopped_at = Some(std::time::SystemTime::now());
        db.save_proxy_config(&updated_config).await?;

        Ok(())
    }

    /// Delete a proxy completely
    pub async fn delete_proxy(&self, proxy_id: &ProxyId) -> AppResult<()> {
        let db = self
            .database
            .read()
            .await
            .as_ref()
            .ok_or_else(|| AppError::proxy("Database not initialized"))?
            .clone();

        // Stop if running
        let mut proxies = self.running_proxies.write().await;
        proxies.remove(&proxy_id.0);

        // Delete from database
        db.delete_proxy_config(&proxy_id.0).await?;

        Ok(())
    }

    /// Get current status of a proxy
    pub async fn get_proxy_status(&self, proxy_id: &ProxyId) -> AppResult<ProxyStatus> {
        let db = self
            .database
            .read()
            .await
            .as_ref()
            .ok_or_else(|| AppError::proxy("Database not initialized"))?
            .clone();

        let config = db
            .get_proxy_config(&proxy_id.0)
            .await?
            .ok_or_else(|| AppError::proxy(format!("Proxy {} not found", proxy_id)))?;

        let proxies = self.running_proxies.read().await;
        let running_info = proxies.get(&proxy_id.0);

        if let Some(info) = running_info {
            let uptime = info.started_at.elapsed().unwrap_or_default().as_secs();
            Ok(ProxyStatus {
                id: proxy_id.clone(),
                name: config.name,
                running: true,
                frontend_url: info.frontend_url.clone(),
                uptime_seconds: Some(uptime),
                total_requests: Some(info.metrics.total_requests),
                error_count: Some(info.metrics.error_count),
                avg_latency_ms: Some(info.metrics.p50_latency_ms),
                last_error: None,
            })
        } else {
            Ok(ProxyStatus {
                id: proxy_id.clone(),
                name: config.name,
                running: false,
                frontend_url: None,
                uptime_seconds: None,
                total_requests: None,
                error_count: None,
                avg_latency_ms: None,
                last_error: None,
            })
        }
    }

    /// List all configured proxies
    pub async fn list_proxies(&self) -> AppResult<Vec<ProxyInfo>> {
        let db = self
            .database
            .read()
            .await
            .as_ref()
            .ok_or_else(|| AppError::proxy("Database not initialized"))?
            .clone();

        let configs = db.list_proxy_configs().await?;
        let running_proxies = self.running_proxies.read().await;

        let mut result = Vec::new();
        for config in configs {
            let running = running_proxies.contains_key(&config.id.0);
            let frontend_url = running_proxies
                .get(&config.id.0)
                .and_then(|info| info.frontend_url.clone());

            result.push(ProxyInfo {
                id: config.id,
                name: config.name,
                description: config.description,
                running,
                frontend_type: config.frontend_type,
                frontend_url,
                created_at: config.created_at,
            });
        }

        Ok(result)
    }

    /// Get live metrics for a proxy
    pub async fn get_proxy_metrics(&self, proxy_id: &ProxyId) -> AppResult<ProxyMetrics> {
        let proxies = self.running_proxies.read().await;
        proxies
            .get(&proxy_id.0)
            .map(|info| info.metrics.clone())
            .ok_or_else(|| AppError::proxy(format!("Proxy {} not running", proxy_id)))
    }

    /// Introspect a backend to discover capabilities
    ///
    /// Uses turbomcp-proxy McpIntrospector for actual server introspection.
    /// Currently supports STDIO backends; HTTP/WebSocket introspection planned.
    pub async fn introspect_backend(
        &self,
        backend_config: &BackendConfig,
        timeout_seconds: Option<u64>,
    ) -> AppResult<ServerSpec> {
        let timeout = std::time::Duration::from_secs(timeout_seconds.unwrap_or(30));

        match backend_config {
            BackendConfig::Stdio { command, args, env: _, working_dir } => {
                tracing::info!("Introspecting STDIO backend: {}", command);

                // Create STDIO backend with turbomcp-proxy
                let mut backend = if let Some(work_dir) = working_dir {
                    StdioBackend::with_working_dir(
                        command.clone(),
                        args.clone().unwrap_or_default(),
                        work_dir.clone(),
                    )
                    .await
                    .map_err(|e| AppError::proxy(format!("Failed to start backend: {}", e)))?
                } else {
                    StdioBackend::new(
                        command.clone(),
                        args.clone().unwrap_or_default(),
                    )
                    .await
                    .map_err(|e| AppError::proxy(format!("Failed to start backend: {}", e)))?
                };

                // Create introspector
                let introspector = McpIntrospector::with_client_info(
                    "TurboMCP Studio",
                    env!("CARGO_PKG_VERSION"),
                );

                // Perform introspection with timeout
                let result = tokio::time::timeout(
                    timeout,
                    introspector.introspect(&mut backend),
                )
                .await
                .map_err(|_| AppError::proxy("Introspection timed out"))?
                .map_err(|e| AppError::proxy(format!("Introspection failed: {}", e)))?;

                // Shutdown backend gracefully
                let _ = backend.shutdown().await;

                // Convert turbomcp-proxy ServerSpec to our local types
                Ok(Self::convert_server_spec(&result))
            }
            BackendConfig::Http { url, headers: _ } => {
                // HTTP introspection not yet implemented in turbomcp-proxy
                tracing::warn!("HTTP introspection not yet available, returning placeholder");
                Ok(ServerSpec {
                    name: format!("HTTP Server at {}", url),
                    version: None,
                    tools: vec![],
                    resources: vec![],
                    prompts: vec![],
                })
            }
            BackendConfig::WebSocket { url, headers: _ } => {
                tracing::warn!("WebSocket introspection not yet available, returning placeholder");
                Ok(ServerSpec {
                    name: format!("WebSocket Server at {}", url),
                    version: None,
                    tools: vec![],
                    resources: vec![],
                    prompts: vec![],
                })
            }
            BackendConfig::Tcp { host, port } => {
                tracing::warn!("TCP introspection not yet available, returning placeholder");
                Ok(ServerSpec {
                    name: format!("TCP Server at {}:{}", host, port),
                    version: None,
                    tools: vec![],
                    resources: vec![],
                    prompts: vec![],
                })
            }
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            BackendConfig::Unix { path } => {
                tracing::warn!("Unix socket introspection not yet available, returning placeholder");
                Ok(ServerSpec {
                    name: format!("Unix Socket at {}", path),
                    version: None,
                    tools: vec![],
                    resources: vec![],
                    prompts: vec![],
                })
            }
        }
    }

    /// Convert turbomcp-proxy ServerSpec to our local types
    fn convert_server_spec(spec: &TurboServerSpec) -> ServerSpec {
        ServerSpec {
            name: spec.server_info.name.clone(),
            version: Some(spec.server_info.version.clone()),
            tools: spec
                .tools
                .iter()
                .map(|t| ToolSpec {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema: serde_json::to_value(&t.input_schema).ok(),
                })
                .collect(),
            resources: spec
                .resources
                .iter()
                .map(|r| ResourceSpec {
                    name: r.name.clone(),
                    uri: r.uri.clone(),
                    description: r.description.clone(),
                    mime_type: r.mime_type.clone(),
                })
                .collect(),
            prompts: spec
                .prompts
                .iter()
                .map(|p| PromptSpec {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    arguments: p
                        .arguments
                        .iter()
                        .map(|a| PromptArgumentSpec {
                            name: a.name.clone(),
                            description: a.description.clone(),
                            required: a.required.unwrap_or(false),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxy_manager_lifecycle() {
        // TODO: Test proxy creation, start, stop, delete
    }
}
