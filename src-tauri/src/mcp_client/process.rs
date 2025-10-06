//! Managed process for STDIO MCP servers

use tokio::process::Child;

/// Managed process for STDIO servers
pub struct ManagedProcess {
    #[allow(dead_code)]
    pub child: Child,
    pub pid: u32,
    #[allow(dead_code)]
    pub started_at: chrono::DateTime<chrono::Utc>,
    #[allow(dead_code)]
    pub command: String,
    #[allow(dead_code)]
    pub args: Vec<String>,
}
