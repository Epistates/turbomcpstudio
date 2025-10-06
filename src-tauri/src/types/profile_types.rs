//! Server Profile Types
//!
//! Type definitions for server profile management.
//! Profiles allow grouping servers for orchestrated startup/shutdown.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server profile - a named collection of servers with orchestration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfile {
    /// Unique profile ID (UUID)
    pub id: String,
    /// Profile name (e.g., "Development", "Production")
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional icon (emoji or icon name)
    pub icon: Option<String>,
    /// Optional color (hex color code)
    pub color: Option<String>,
    /// Whether to auto-activate on startup
    pub auto_activate: bool,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last update timestamp (ISO 8601)
    pub updated_at: String,
}

/// Request to create or update a server profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProfileRequest {
    /// Profile name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional icon (emoji or icon name)
    pub icon: Option<String>,
    /// Optional color (hex color code)
    pub color: Option<String>,
    /// Whether to auto-activate on startup
    #[serde(default)]
    pub auto_activate: bool,
}

/// Server profile with server count
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfileWithCount {
    /// The profile
    pub profile: ServerProfile,
    /// Number of servers in the profile
    pub server_count: i32,
}

/// Configuration for a server within a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileServerConfig {
    /// Startup order (lower numbers start first)
    #[serde(default)]
    pub startup_order: i32,
    /// Delay in milliseconds after starting before starting next
    #[serde(default)]
    pub startup_delay_ms: i32,
    /// Whether to auto-connect this server when profile is activated
    #[serde(default = "default_true")]
    pub auto_connect: bool,
    /// Whether to auto-restart this server if it fails
    #[serde(default)]
    pub auto_restart: bool,
    /// Whether this server is required (profile activation fails if it can't connect)
    #[serde(default)]
    pub required: bool,
    /// Environment variable overrides specific to this profile
    pub environment_overrides: Option<HashMap<String, String>>,
}

fn default_true() -> bool {
    true
}

/// Request to add a server to a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddServerToProfileRequest {
    /// Profile ID (UUID)
    pub profile_id: String,
    /// Server ID (UUID)
    pub server_id: String,
    /// Server configuration within the profile
    pub config: ProfileServerConfig,
}

/// A server within a profile (includes server metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileServer {
    /// Profile ID
    pub profile_id: String,
    /// Server ID
    pub server_id: String,
    /// Server name (from server_configs table)
    pub server_name: String,
    /// Server description (from server_configs table)
    pub server_description: Option<String>,
    /// Transport type (from server_configs table)
    pub transport_type: String,
    /// Startup order
    pub startup_order: i32,
    /// Startup delay in milliseconds
    pub startup_delay_ms: i32,
    /// Auto-connect on profile activation
    pub auto_connect: bool,
    /// Auto-restart on failure
    pub auto_restart: bool,
    /// Required for profile activation
    pub required: bool,
    /// Environment variable overrides
    pub environment_overrides: Option<HashMap<String, String>>,
    /// When this server was added to the profile
    pub created_at: String,
}

/// Profile activation record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileActivation {
    /// Activation ID (UUID)
    pub id: String,
    /// Profile ID
    pub profile_id: String,
    /// Profile name
    pub profile_name: String,
    /// When the profile was activated (ISO 8601)
    pub activated_at: String,
    /// When the profile was deactivated (ISO 8601), if applicable
    pub deactivated_at: Option<String>,
    /// Number of servers successfully connected
    pub success_count: i32,
    /// Number of servers that failed to connect
    pub failure_count: i32,
    /// Error messages from failed connections
    pub errors: Option<Vec<String>>,
}

/// Current active profile state (for UI)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveProfileState {
    /// The active profile (if any)
    pub profile: Option<ServerProfile>,
    /// Servers in the profile
    pub servers: Vec<ProfileServer>,
    /// Latest activation record
    pub activation: Option<ProfileActivation>,
    /// Whether profile is currently being activated
    pub is_activating: bool,
}
