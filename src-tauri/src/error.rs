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
    McpError(#[from] turbomcp_protocol::McpError),

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

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Proxy error: {0}")]
    ProxyError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias for MCP Studio operations
pub type McpResult<T> = Result<T, McpStudioError>;

/// Application error (alias for McpStudioError)
pub type AppError = McpStudioError;

/// Application result type (alias for McpResult)
pub type AppResult<T> = McpResult<T>;

impl McpStudioError {
    /// Create a proxy error
    pub fn proxy(msg: impl Into<String>) -> Self {
        Self::ProxyError(msg.into())
    }
}

impl serde::Serialize for McpStudioError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        #[derive(serde::Serialize)]
        struct ErrorPayload<'a> {
            error_type: &'a str,
            message: String,
        }

        let error_type = match self {
            Self::ConnectionFailed(_) => "ConnectionFailed",
            Self::ProcessError(_) => "ProcessError",
            Self::ProcessSpawn(_) => "ProcessSpawn",
            Self::ToolCallFailed(_) => "ToolCallFailed",
            Self::UnsupportedTransport(_) => "UnsupportedTransport",
            Self::DatabaseError(_) => "DatabaseError",
            Self::SerializationError(_) => "SerializationError",
            Self::TurboMcpError(_) => "TurboMcpError",
            Self::McpError(_) => "McpError",
            Self::IoError(_) => "IoError",
            Self::TauriError(_) => "TauriError",
            Self::ConfigError(_) => "ConfigError",
            Self::ServerNotFound(_) => "ServerNotFound",
            Self::CollectionNotFound(_) => "CollectionNotFound",
            Self::ScenarioError(_) => "ScenarioError",
            Self::WorkflowError(_) => "WorkflowError",
            Self::ProtocolError(_) => "ProtocolError",
            Self::AuthError(_) => "AuthError",
            Self::PermissionDenied(_) => "PermissionDenied",
            Self::ResourceUnavailable(_) => "ResourceUnavailable",
            Self::TimeoutError(_) => "TimeoutError",
            Self::RateLimitExceeded(_) => "RateLimitExceeded",
            Self::ValidationError(_) => "ValidationError",
            Self::ProxyError(_) => "ProxyError",
            Self::Unknown(_) => "Unknown",
        };

        ErrorPayload {
            error_type,
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}
