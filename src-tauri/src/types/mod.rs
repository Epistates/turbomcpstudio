pub mod collections;
pub mod llm_config;
pub mod profile_types;
pub mod server_types;

pub use llm_config::*;
pub use profile_types::*;
pub use server_types::*;

/// Represents an error that occurred during application initialization
/// Emitted to frontend to display user-friendly error messages instead of silent hangs
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationError {
    /// Whether this error prevents the app from functioning (true = blocking dialog)
    pub critical: bool,
    /// Component that failed (e.g., "filesystem", "database", "config")
    pub component: String,
    /// Human-readable error message
    pub message: String,
    /// Optional fallback that was used (e.g., "in-memory database")
    pub fallback_used: Option<String>,
    /// Optional user action suggestion (e.g., "Check folder permissions")
    pub user_action: Option<String>,
}
