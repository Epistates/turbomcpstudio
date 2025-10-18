//! Client Installation Commands
//!
//! Commands for installing MCP servers from TurboMCPStudio to other applications:
//! - Claude Desktop
//! - Claude Code
//! - LM Studio
//! - Continue.dev
//! - Cursor
//! - Codex
//! - Cline (VS Code extension)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Supported client applications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ClientApp {
    ClaudeDesktop,
    ClaudeCode,
    LMStudio,
    ContinueDev,
    Cursor,
    Codex,
    Cline,
}

impl ClientApp {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ClaudeDesktop => "Claude Desktop",
            Self::ClaudeCode => "Claude Code",
            Self::LMStudio => "LM Studio",
            Self::ContinueDev => "Continue.dev",
            Self::Cursor => "Cursor",
            Self::Codex => "Codex",
            Self::Cline => "Cline",
        }
    }

    /// Get the config file path for this application on the current OS
    pub fn config_path(&self) -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;

        let path = match self {
            // Claude Desktop
            Self::ClaudeDesktop => {
                #[cfg(target_os = "macos")]
                {
                    home.join("Library/Application Support/Claude/claude_desktop_config.json")
                }
                #[cfg(target_os = "windows")]
                {
                    let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not set".to_string())?;
                    PathBuf::from(&appdata).join("Claude\\claude_desktop_config.json")
                }
                #[cfg(target_os = "linux")]
                {
                    home.join(".config/Claude/claude_desktop_config.json")
                }
            }

            // Claude Code
            Self::ClaudeCode => {
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                {
                    home.join(".claude.json")
                }
                #[cfg(target_os = "windows")]
                {
                    PathBuf::from(std::env::var("USERPROFILE").map_err(|_| "USERPROFILE not set".to_string())?).join(".claude.json")
                }
            }

            // LM Studio
            Self::LMStudio => {
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                {
                    home.join(".lmstudio/mcp.json")
                }
                #[cfg(target_os = "windows")]
                {
                    let userprofile = std::env::var("USERPROFILE").map_err(|_| "USERPROFILE not set".to_string())?;
                    PathBuf::from(&userprofile).join(".lmstudio/mcp.json")
                }
            }

            // Cursor (global config)
            Self::Cursor => {
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                {
                    home.join(".cursor/mcp.json")
                }
                #[cfg(target_os = "windows")]
                {
                    let userprofile = std::env::var("USERPROFILE").map_err(|_| "USERPROFILE not set".to_string())?;
                    PathBuf::from(&userprofile).join(".cursor/mcp.json")
                }
            }

            // Codex
            Self::Codex => {
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                {
                    home.join(".codex/config.toml")
                }
                #[cfg(target_os = "windows")]
                {
                    let userprofile = std::env::var("USERPROFILE").map_err(|_| "USERPROFILE not set".to_string())?;
                    PathBuf::from(&userprofile).join(".codex/config.toml")
                }
            }

            // Cline (VS Code extension)
            Self::Cline => {
                #[cfg(target_os = "macos")]
                {
                    home.join("Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
                }
                #[cfg(target_os = "windows")]
                {
                    let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not set".to_string())?;
                    PathBuf::from(&appdata).join("Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
                }
                #[cfg(target_os = "linux")]
                {
                    home.join(".config/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
                }
            }

            // Continue.dev - workspace-relative (handled separately)
            Self::ContinueDev => {
                return Err("Continue.dev uses workspace-relative paths (.continue/config.yaml)".to_string());
            }
        };

        Ok(path)
    }
}

/// Application detection response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDetectionResult {
    pub app: String,
    pub installed: bool,
    pub config_path: Option<String>,
}

/// Server to install
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToInstall {
    pub name: String,
    pub config: serde_json::Value,
}

/// Installation result for a single app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationResult {
    pub app: String,
    pub success: bool,
    pub message: String,
    pub servers_added: usize,
    pub servers_updated: usize,
}

/// Detect which client applications are installed
#[tauri::command]
pub async fn detect_installed_clients() -> Result<Vec<AppDetectionResult>, String> {
    let mut results = Vec::new();

    for app in &[
        ClientApp::ClaudeDesktop,
        ClientApp::ClaudeCode,
        ClientApp::LMStudio,
        ClientApp::Cursor,
        ClientApp::Codex,
        ClientApp::Cline,
    ] {
        if let Ok(path) = app.config_path() {
            let installed = path.exists();
            results.push(AppDetectionResult {
                app: app.name().to_string(),
                installed,
                config_path: if installed {
                    Some(path.to_string_lossy().to_string())
                } else {
                    None
                },
            });
        } else {
            results.push(AppDetectionResult {
                app: app.name().to_string(),
                installed: false,
                config_path: None,
            });
        }
    }

    Ok(results)
}

/// Install servers to a specific client application
#[tauri::command]
pub async fn install_servers_to_client(
    app_name: String,
    servers: Vec<ServerToInstall>,
) -> Result<InstallationResult, String> {
    // Find the matching app
    let app = match app_name.as_str() {
        "Claude Desktop" => ClientApp::ClaudeDesktop,
        "Claude Code" => ClientApp::ClaudeCode,
        "LM Studio" => ClientApp::LMStudio,
        "Continue.dev" => ClientApp::ContinueDev,
        "Cursor" => ClientApp::Cursor,
        "Codex" => ClientApp::Codex,
        "Cline" => ClientApp::Cline,
        _ => return Err(format!("Unknown application: {}", app_name)),
    };

    if servers.is_empty() {
        return Err("No servers to install".to_string());
    }

    match app {
        ClientApp::ContinueDev => {
            Err("Continue.dev installation not yet implemented (workspace-relative paths)".to_string())
        }
        _ => install_to_json_config(&app, &servers),
    }
}

/// Check if a server transport is compatible with standard MCP clients
fn is_supported_transport(transport_type: &str) -> bool {
    // Only allow standard MCP transports: stdio, http, webSocket
    // Exclude: tcp, unix (TurboMCP extensions, not universally supported)
    matches!(transport_type, "stdio" | "http" | "webSocket")
}

/// Check if two server configs are exactly equivalent
fn configs_are_equivalent(config1: &serde_json::Value, config2: &serde_json::Value) -> bool {
    // Compare JSON values - if they're identical, don't update
    config1 == config2
}

/// Install to JSON-based config files (most apps)
fn install_to_json_config(
    app: &ClientApp,
    servers: &[ServerToInstall],
) -> Result<InstallationResult, String> {
    let config_path = app.config_path()?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Read existing config or create new one
    let mut config: serde_json::Value = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        serde_json::from_str(&content).unwrap_or_else(|_| {
            serde_json::json!({ "mcpServers": {} })
        })
    } else {
        serde_json::json!({ "mcpServers": {} })
    };

    // Ensure mcpServers object exists
    if config["mcpServers"].is_null() {
        config["mcpServers"] = serde_json::json!({});
    }

    let mut added = 0;
    let mut updated = 0;
    let mut skipped = 0;
    let mut unsupported = 0;

    // Intelligently merge servers - be a good neighbor
    for server in servers {
        // Determine transport type and check if it's supported
        let is_supported = if server.config.get("command").is_some() {
            is_supported_transport("stdio")
        } else if server.config.get("url").is_some() {
            if server.config.get("transport").and_then(|v| v.as_str()) == Some("websocket") {
                is_supported_transport("webSocket")
            } else {
                is_supported_transport("http")
            }
        } else if server.config.get("host").is_some() {
            // TCP transport - not supported
            false
        } else if server.config.get("path").is_some() {
            // Unix socket - not supported
            false
        } else {
            false
        };

        if !is_supported {
            // Transport not supported - skip it
            unsupported += 1;
            continue;
        }

        let existing = &config["mcpServers"][&server.name];

        if existing.is_null() {
            // Server doesn't exist - add it
            config["mcpServers"][&server.name] = server.config.clone();
            added += 1;
        } else if configs_are_equivalent(existing, &server.config) {
            // Server exists with identical config - skip it
            skipped += 1;
        } else {
            // Server exists with different config - update it
            config["mcpServers"][&server.name] = server.config.clone();
            updated += 1;
        }
    }

    // Only backup and write if there are actual changes
    if added > 0 || updated > 0 {
        // Backup original file before modifying
        if config_path.exists() {
            let backup_path = config_path.with_extension(
                format!("json.backup.{}", chrono::Local::now().format("%Y%m%d_%H%M%S"))
            );
            fs::copy(&config_path, &backup_path)
                .map_err(|e| format!("Failed to create backup: {}", e))?;
        }

        // Write updated config
        let json_str = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, json_str)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
    }

    // Build a smart message
    let mut message = format!("Successfully processed servers for {}", app.name());
    let mut details = Vec::new();
    if added > 0 {
        details.push(format!("{} added", added));
    }
    if updated > 0 {
        details.push(format!("{} updated", updated));
    }
    if skipped > 0 {
        details.push(format!("{} already present", skipped));
    }
    if unsupported > 0 {
        details.push(format!("{} excluded (non-standard transport)", unsupported));
    }
    if !details.is_empty() {
        message.push_str(&format!(" ({})", details.join(", ")));
    }

    Ok(InstallationResult {
        app: app.name().to_string(),
        success: true,
        message,
        servers_added: added,
        servers_updated: updated,
    })
}
