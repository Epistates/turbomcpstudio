use thiserror::Error;

/// Main error type for MCP Studio operations
#[derive(Error, Debug)]
pub enum McpStudioError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Process management error: {0}")]
    ProcessError(String),

    #[error("Process spawn error: {0}")]
    ProcessSpawn(String),

    #[error("Tool call failed: {0}")]
    ToolCallFailed(String),

    #[error("Unsupported transport: {0}")]
    UnsupportedTransport(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("TurboMCP error: {0}")]
    TurboMcpError(String),

    #[error("MCP protocol error: {0}")]
    McpError(#[from] turbomcp::McpError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Tauri error: {0}")]
    TauriError(#[from] tauri::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Scenario execution error: {0}")]
    ScenarioError(String),

    #[error("Workflow execution error: {0}")]
    WorkflowError(String),

    #[error("Protocol validation error: {0}")]
    ProtocolError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource not available: {0}")]
    ResourceUnavailable(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias for MCP Studio operations
pub type McpResult<T> = Result<T, McpStudioError>;

impl serde::Serialize for McpStudioError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
