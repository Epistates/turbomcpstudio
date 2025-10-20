//! Configuration generator for various MCP clients

use super::platform::normalize_volume_mount;
use super::types::*;
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
    ClaudeDesktop,
    ClaudeCode,
    LMStudio,
    Cursor,
    Cline,
    TurboMCP,
}

impl ClientType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ClaudeDesktop => "Claude Desktop",
            Self::ClaudeCode => "Claude Code",
            Self::LMStudio => "LM Studio",
            Self::Cursor => "Cursor",
            Self::Cline => "Cline",
            Self::TurboMCP => "TurboMCP Studio",
        }
    }
}

/// User-provided configuration values
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserConfig {
    pub parameters: HashMap<String, serde_json::Value>,
    pub secrets: HashMap<String, String>,
}

/// Generated configuration ready for a specific client
#[derive(Debug, Clone, serde::Serialize)]
pub struct GeneratedConfig {
    pub client_type: String,
    pub server_name: String,
    pub config_json: String,
    pub notes: Vec<String>,
}

/// Generates configuration for a specific client type
pub fn generate_config(
    server: &RegistryServer,
    user_config: &UserConfig,
    client_type: ClientType,
) -> Result<GeneratedConfig> {
    let mut notes = Vec::new();

    let config_value = match (client_type, &server.server_type) {
        // Container-based servers
        (ClientType::ClaudeDesktop, ServerType::Server) => {
            generate_claude_desktop_docker(server, user_config, &mut notes)?
        }
        (ClientType::ClaudeCode, ServerType::Server) => {
            generate_claude_code_docker(server, user_config, &mut notes)?
        }
        (ClientType::LMStudio, ServerType::Server) => {
            generate_lmstudio_docker(server, user_config, &mut notes)?
        }
        (ClientType::Cursor, ServerType::Server) => {
            generate_cursor_docker(server, user_config, &mut notes)?
        }
        (ClientType::Cline, ServerType::Server) => {
            generate_cline_docker(server, user_config, &mut notes)?
        }
        (ClientType::TurboMCP, ServerType::Server) => {
            generate_turbomcp_docker(server, user_config, &mut notes)?
        }

        // Remote servers
        (ClientType::ClaudeDesktop, ServerType::Remote) => {
            generate_claude_desktop_remote(server, user_config, &mut notes)?
        }
        (ClientType::ClaudeCode, ServerType::Remote) => {
            generate_claude_code_remote(server, user_config, &mut notes)?
        }
        (ClientType::LMStudio, ServerType::Remote) => {
            generate_lmstudio_remote(server, user_config, &mut notes)?
        }
        (ClientType::Cursor, ServerType::Remote) => {
            generate_cursor_remote(server, user_config, &mut notes)?
        }
        (ClientType::Cline, ServerType::Remote) => {
            generate_cline_remote(server, user_config, &mut notes)?
        }
        (ClientType::TurboMCP, ServerType::Remote) => {
            generate_turbomcp_remote(server, user_config, &mut notes)?
        }
    };

    let config_json =
        serde_json::to_string_pretty(&config_value).context("Failed to serialize config")?;

    Ok(GeneratedConfig {
        client_type: client_type.name().to_string(),
        server_name: server.name.clone(),
        config_json,
        notes,
    })
}

/// Claude Desktop - Docker container
fn generate_claude_desktop_docker(
    server: &RegistryServer,
    user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let image = server
        .image
        .as_ref()
        .context("Server has no Docker image")?;

    let mut args = vec!["run".to_string(), "-i".to_string(), "--rm".to_string()];

    // Add environment variables
    let env_vars = build_env_vars(server, user_config, notes)?;

    // Add volumes with cross-platform path normalization
    if let Some(run) = &server.run {
        if let Some(volumes) = &run.volumes {
            for volume in volumes {
                let resolved = resolve_template(volume, &server.name, &user_config.parameters);
                // Normalize paths for cross-platform compatibility (Windows: C:\ -> /c/)
                let normalized = normalize_volume_mount(&resolved);
                args.push("-v".to_string());
                args.push(normalized);
            }
        }

        // Add command overrides
        if let Some(command) = &run.command {
            for arg in command {
                let resolved = resolve_template(arg, &server.name, &user_config.parameters);
                args.push(resolved);
            }
        }
    }

    args.push(image.clone());

    Ok(json!({
        "mcpServers": {
            server.name.clone(): {
                "command": "docker",
                "args": args,
                "env": env_vars,
            }
        }
    }))
}

/// Claude Code - Docker container
fn generate_claude_code_docker(
    server: &RegistryServer,
    user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    // Claude Code uses same format as Claude Desktop
    let config = generate_claude_desktop_docker(server, user_config, notes)?;
    Ok(config["mcpServers"].clone())
}

/// LM Studio - Docker container
fn generate_lmstudio_docker(
    server: &RegistryServer,
    user_config: &UserConfig,
    _notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let image = server
        .image
        .as_ref()
        .context("Server has no Docker image")?;

    let mut args = vec!["run".to_string(), "-i".to_string(), "--rm".to_string()];

    // Add volumes with cross-platform path normalization
    if let Some(run) = &server.run {
        if let Some(volumes) = &run.volumes {
            for volume in volumes {
                let resolved = resolve_template(volume, &server.name, &user_config.parameters);
                // Normalize paths for cross-platform compatibility (Windows: C:\ -> /c/)
                let normalized = normalize_volume_mount(&resolved);
                args.push("-v".to_string());
                args.push(normalized);
            }
        }

        // Add command overrides
        if let Some(command) = &run.command {
            for arg in command {
                let resolved = resolve_template(arg, &server.name, &user_config.parameters);
                args.push(resolved);
            }
        }
    }

    args.push(image.clone());

    Ok(json!({
        server.name.clone(): {
            "command": "docker",
            "args": args,
        }
    }))
}

/// Cursor - Docker container
fn generate_cursor_docker(
    server: &RegistryServer,
    user_config: &UserConfig,
    _notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let mut temp_notes = Vec::new();
    generate_lmstudio_docker(server, user_config, &mut temp_notes)
}

/// Cline - Docker container
fn generate_cline_docker(
    server: &RegistryServer,
    user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    generate_claude_desktop_docker(server, user_config, notes)
}

/// Remote servers - Claude Desktop
fn generate_claude_desktop_remote(
    server: &RegistryServer,
    _user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let remote = server
        .remote
        .as_ref()
        .context("Server has no remote configuration")?;

    let url = remote.url.as_ref().context("Remote server has no URL")?;

    notes.push("Remote MCP servers don't require Docker".to_string());
    notes.push(format!("This server connects to: {}", url));

    if server.oauth.as_ref().is_some_and(|o| !o.is_empty()) {
        notes.push("⚠️  This server requires OAuth authentication".to_string());
    }

    Ok(json!({
        "mcpServers": {
            server.name.clone(): {
                "url": url,
            }
        }
    }))
}

/// Remote servers - Claude Code
fn generate_claude_code_remote(
    server: &RegistryServer,
    user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let config = generate_claude_desktop_remote(server, user_config, notes)?;
    Ok(config["mcpServers"].clone())
}

/// Remote servers - LM Studio
fn generate_lmstudio_remote(
    server: &RegistryServer,
    _user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let remote = server
        .remote
        .as_ref()
        .context("Server has no remote configuration")?;

    let url = remote.url.as_ref().context("Remote server has no URL")?;

    notes.push("Remote MCP servers connect via HTTP".to_string());

    Ok(json!({
        server.name.clone(): {
            "url": url,
        }
    }))
}

/// Remote servers - Cursor
fn generate_cursor_remote(
    server: &RegistryServer,
    user_config: &UserConfig,
    _notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let mut temp_notes = Vec::new();
    generate_lmstudio_remote(server, user_config, &mut temp_notes)
}

/// Remote servers - Cline
fn generate_cline_remote(
    server: &RegistryServer,
    user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    generate_claude_desktop_remote(server, user_config, notes)
}

/// Builds environment variable map from server config and user input
fn build_env_vars(
    server: &RegistryServer,
    user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    if let Some(config) = &server.config {
        // Add regular environment variables
        if let Some(env_list) = &config.env {
            for env in env_list {
                if let Some(value_template) = &env.value {
                    let resolved =
                        resolve_template(value_template, &server.name, &user_config.parameters);
                    env_vars.insert(env.name.clone(), resolved);
                }
            }
        }

        // Add secrets
        if let Some(secrets) = &config.secrets {
            for secret in secrets {
                if let Some(value) = user_config.secrets.get(&secret.name) {
                    env_vars.insert(secret.env.clone(), value.clone());
                } else if secret.required.unwrap_or(false) {
                    notes.push(format!("⚠️  Required secret missing: {}", secret.name));
                }
            }
        }
    }

    Ok(env_vars)
}

/// TurboMCP Studio - Docker container (native TurboMCP format)
fn generate_turbomcp_docker(
    server: &RegistryServer,
    user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let image = server
        .image
        .as_ref()
        .context("Server has no Docker image")?;

    let mut args = vec!["run".to_string(), "-i".to_string(), "--rm".to_string()];

    // Add environment variables
    let env_vars = build_env_vars(server, user_config, notes)?;

    // Add environment variables as docker run flags
    for (key, value) in &env_vars {
        args.push("-e".to_string());
        args.push(format!("{}={}", key, value));
    }

    // Add volumes with cross-platform path normalization
    if let Some(run) = &server.run {
        if let Some(volumes) = &run.volumes {
            for volume in volumes {
                let resolved = resolve_template(volume, &server.name, &user_config.parameters);
                // Normalize paths for cross-platform compatibility (Windows: C:\ -> /c/)
                let normalized = normalize_volume_mount(&resolved);
                args.push("-v".to_string());
                args.push(normalized);
            }
        }

        // Add command overrides
        if let Some(command) = &run.command {
            for arg in command {
                let resolved = resolve_template(arg, &server.name, &user_config.parameters);
                args.push(resolved);
            }
        }
    }

    args.push(image.clone());

    notes.push("TurboMCP Studio uses stdio transport for Docker containers".to_string());
    notes.push("This server will appear in your local server list".to_string());

    // Return TurboMCP native format
    Ok(json!({
        "name": server.name,
        "description": server.about.as_ref().and_then(|a| a.description.clone()),
        "transport_config": {
            "type": "stdio",
            "command": "docker",
            "args": args
        },
        "environment_variables": {}
    }))
}

/// TurboMCP Studio - Remote server (native TurboMCP format)
fn generate_turbomcp_remote(
    server: &RegistryServer,
    _user_config: &UserConfig,
    notes: &mut Vec<String>,
) -> Result<serde_json::Value> {
    let remote = server
        .remote
        .as_ref()
        .context("Server has no remote configuration")?;

    let url = remote.url.as_ref().context("Remote server has no URL")?;

    notes.push("TurboMCP Studio supports HTTP and WebSocket remote servers".to_string());
    notes.push(format!("This server connects to: {}", url));

    if server.oauth.as_ref().is_some_and(|o| !o.is_empty()) {
        notes.push("⚠️  This server requires OAuth authentication".to_string());
        notes.push("⚠️  OAuth support may be limited in TurboMCP Studio".to_string());
    }

    // Determine transport type from URL
    let transport_type = if url.starts_with("ws://") || url.starts_with("wss://") {
        "webSocket"
    } else {
        "http"
    };

    // Return TurboMCP native format
    Ok(json!({
        "name": server.name,
        "description": server.about.as_ref().and_then(|a| a.description.clone()),
        "transport_config": {
            "type": transport_type,
            "url": url,
            "headers": {}
        },
        "environment_variables": {}
    }))
}

/// Resolves template variables like {{server_name.parameter_name}}
fn resolve_template(
    template: &str,
    server_name: &str,
    parameters: &HashMap<String, serde_json::Value>,
) -> String {
    let mut result = template.to_string();

    // Find all {{...}} patterns
    let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();

    for cap in re.captures_iter(template) {
        if let Some(matched) = cap.get(1) {
            let var_path = matched.as_str().trim();

            // Extract parameter name from path like "server_name.param_name"
            if let Some(param_name) = var_path.strip_prefix(&format!("{}.", server_name)) {
                if let Some(value) = parameters.get(param_name) {
                    let replacement = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Array(arr) => {
                            // For arrays, join with colons (common in volume paths)
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(":")
                        }
                        _ => continue,
                    };

                    result = result.replace(&format!("{{{{{}}}}}", var_path), &replacement);
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_resolution() {
        let mut params = HashMap::new();
        params.insert("db_path".to_string(), json!("/Users/test/db"));

        let result = resolve_template("{{sqlite.db_path}}:/data/db.sqlite", "sqlite", &params);

        assert_eq!(result, "/Users/test/db:/data/db.sqlite");
    }

    #[test]
    fn test_array_template_resolution() {
        let mut params = HashMap::new();
        params.insert("paths".to_string(), json!(["/path1", "/path2"]));

        let result = resolve_template("{{filesystem.paths}}", "filesystem", &params);

        assert_eq!(result, "/path1:/path2");
    }
}
