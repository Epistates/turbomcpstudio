//! Managed process for STDIO MCP servers

use tokio::process::Child;

/// Managed process for STDIO servers
pub struct ManagedProcess {
    /// Child process handle. Held to keep the process alive for its lifetime.
    /// The OS process is terminated when this is dropped.
    #[allow(dead_code)]
    pub child: Child,
    pub pid: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub command: String,
    pub args: Vec<String>,
}
