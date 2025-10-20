//! Type definitions for Docker MCP Registry
//!
//! Matches the schema defined in:
//! https://github.com/docker/mcp-registry/blob/main/pkg/servers/types.go
//! https://github.com/docker/mcp-registry/blob/main/pkg/catalog/types.go

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A server definition from the Docker MCP registry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryServer {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(rename = "type")]
    pub server_type: ServerType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<Dynamic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_lived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ServerMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<About>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<Remote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<Run>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ServerConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<Vec<OAuthProvider>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServerType {
    Server,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dynamic {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct About {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_hosts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_network: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<Secret>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<Env>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<JsonSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<AnyOf>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub name: String,
    pub env: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Env {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnyOf {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// JSON Schema definition for configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, JsonSchemaProperty>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<AnyOf>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaProperty {
    #[serde(rename = "type")]
    pub property_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<JsonSchemaItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaItems {
    #[serde(rename = "type")]
    pub item_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<ToolParameters>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: HashMap<String, ToolProperty>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProperty {
    #[serde(rename = "type")]
    pub property_type: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<JsonSchemaItems>,
}

/// Registry catalog containing all servers
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryCatalog {
    pub version: i32,
    pub name: String,
    pub display_name: String,
    pub registry: HashMap<String, RegistryServer>,
}

/// Metadata for displaying servers in UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDisplayInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    pub icon: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub server_type: ServerType,
    pub is_docker_built: bool,
    pub is_remote: bool,
    pub has_oauth: bool,
    pub github_url: Option<String>,
}

impl From<&RegistryServer> for ServerDisplayInfo {
    fn from(server: &RegistryServer) -> Self {
        let about = server.about.as_ref();
        let meta = server.meta.as_ref();
        let source = server.source.as_ref();

        ServerDisplayInfo {
            name: server.name.clone(),
            title: about
                .and_then(|a| a.title.clone())
                .unwrap_or_else(|| server.name.clone()),
            description: about
                .and_then(|a| a.description.clone())
                .unwrap_or_default(),
            icon: about.and_then(|a| a.icon.clone()),
            category: meta
                .and_then(|m| m.category.clone())
                .unwrap_or_else(|| "other".to_string()),
            tags: meta.and_then(|m| m.tags.clone()).unwrap_or_default(),
            server_type: server.server_type.clone(),
            is_docker_built: server
                .image
                .as_ref()
                .is_some_and(|img| img.starts_with("mcp/")),
            is_remote: server.server_type == ServerType::Remote,
            has_oauth: server.oauth.as_ref().is_some_and(|o| !o.is_empty()),
            github_url: source.and_then(|s| s.project.clone()),
        }
    }
}
