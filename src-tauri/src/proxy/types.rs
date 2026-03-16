//! Type definitions for proxy functionality

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Unique identifier for a proxy instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProxyId(pub String);

impl ProxyId {
    /// Generate a new random proxy ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for ProxyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProxyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Backend transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackendConfig {
    /// Standard I/O subprocess
    Stdio {
        command: String,
        #[serde(default)]
        args: Option<Vec<String>>,
        #[serde(default)]
        env: Option<std::collections::HashMap<String, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        working_dir: Option<String>,
    },
    /// HTTP/SSE backend
    Http {
        url: String,
        #[serde(default)]
        headers: Option<std::collections::HashMap<String, String>>,
    },
    /// TCP socket backend
    Tcp { host: String, port: u16 },
    /// Unix domain socket backend
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Unix { path: String },
    /// WebSocket backend
    WebSocket {
        url: String,
        #[serde(default)]
        headers: Option<std::collections::HashMap<String, String>>,
    },
}

/// Frontend exposure type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrontendType {
    /// HTTP with Server-Sent Events
    Http,
    /// WebSocket bidirectional
    WebSocket,
    /// TCP socket
    Tcp,
}

impl std::fmt::Display for FrontendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http => write!(f, "HTTP"),
            Self::WebSocket => write!(f, "WebSocket"),
            Self::Tcp => write!(f, "TCP"),
        }
    }
}

/// Authentication configuration for backend
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    /// No authentication
    #[default]
    None,
    /// Bearer token
    Bearer { token: String },
    /// API key in header
    ApiKey { key: String, header: String },
    /// JWT signing
    Jwt {
        issuer: String,
        audience: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        secret: Option<String>,
    },
}

/// Proxy configuration (persisted to database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub id: ProxyId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub backend_type: String, // Serialized backend config
    pub backend_config: serde_json::Value,

    pub frontend_type: FrontendType,
    pub frontend_config: serde_json::Value,

    #[serde(default)]
    pub auth_config: AuthConfig,

    #[serde(default = "default_true")]
    pub metrics_enabled: bool,

    pub max_requests_tracked: usize,

    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_started_at: Option<SystemTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_stopped_at: Option<SystemTime>,
}

fn default_true() -> bool {
    true
}

/// Runtime status of a proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub id: ProxyId,
    pub name: String,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_requests: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Metrics for a running proxy
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyMetrics {
    pub total_requests: u64,
    pub error_count: u64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub uptime_seconds: u64,
}

/// Server specification from introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub tools: Vec<ToolSpec>,
    pub resources: Vec<ResourceSpec>,
    pub prompts: Vec<PromptSpec>,
}

/// Tool specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

/// Resource specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSpec {
    pub name: String,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Prompt specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: Vec<PromptArgumentSpec>,
}

/// Prompt argument specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgumentSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

/// Proxy list item (for UI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    pub id: ProxyId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub running: bool,
    pub frontend_type: FrontendType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_url: Option<String>,
    pub created_at: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_id_generation() {
        let id1 = ProxyId::new();
        let id2 = ProxyId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_proxy_config_serialization() {
        let config = ProxyConfig {
            id: ProxyId::new(),
            name: "test".to_string(),
            description: None,
            backend_type: "stdio".to_string(),
            backend_config: serde_json::json!({}),
            frontend_type: FrontendType::Http,
            frontend_config: serde_json::json!({}),
            auth_config: AuthConfig::None,
            metrics_enabled: true,
            max_requests_tracked: 10000,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            last_started_at: None,
            last_stopped_at: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let restored: ProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, config.id);
    }
}
