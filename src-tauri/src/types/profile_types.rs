use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server Profile - Groups of servers for different environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub auto_activate: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Server Profile with server count (for list views)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfileWithCount {
    #[serde(flatten)]
    pub profile: ServerProfile,
    pub server_count: i64,
}

/// Configuration for a server within a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileServerConfig {
    pub startup_order: i32,
    pub startup_delay_ms: i32,
    pub auto_connect: bool,
    pub auto_restart: bool,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_overrides: Option<HashMap<String, String>>,
}

impl Default for ProfileServerConfig {
    fn default() -> Self {
        Self {
            startup_order: 0,
            startup_delay_ms: 0,
            auto_connect: true,
            auto_restart: false,
            required: false,
            environment_overrides: None,
        }
    }
}

/// Server within a profile (includes server config and profile-specific settings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileServer {
    pub profile_id: String,
    pub server_id: String,
    pub server_name: String,
    pub server_description: Option<String>,
    pub transport_type: String,
    pub startup_order: i32,
    pub startup_delay_ms: i32,
    pub auto_connect: bool,
    pub auto_restart: bool,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_overrides: Option<HashMap<String, String>>,
    pub created_at: String,
}

/// Profile activation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileActivation {
    pub id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub activated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deactivated_at: Option<String>,
    pub success_count: i32,
    pub failure_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

/// Active profile state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProfileState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServerProfile>,
    pub servers: Vec<ProfileServer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation: Option<ProfileActivation>,
    pub is_activating: bool,
}

/// Request to create or update a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub auto_activate: bool,
}

/// Request to add a server to a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddServerToProfileRequest {
    pub profile_id: String,
    pub server_id: String,
    #[serde(flatten)]
    pub config: ProfileServerConfig,
}

/// Profile export format (for sharing)
/// TODO(phase-5): Implement profile export/import functionality with version handling
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExport {
    pub profile: ServerProfile,
    pub servers: Vec<ProfileServer>,
}

/// Profile template
/// TODO(phase-5): Implement profile template system for quick workspace setup
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub servers: Vec<TemplateServer>,
}

/// Server definition in a template
/// TODO(phase-5): Used by ProfileTemplate for pre-configured server setups
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateServer {
    pub name: String,
    pub description: String,
    pub transport_type: String,
    pub config: ProfileServerConfig,
}
