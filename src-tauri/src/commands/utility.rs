//! Utility Commands
//!
//! Tauri commands for utility functions.
//! - Open URLs in browser
//! - Fetch LLM models
//! - Make LLM completion requests

use serde_json::Value;
use tauri::Manager;

use serde::{Deserialize, Serialize};

/// Validate a base URL intended for LLM API requests.
///
/// Enforces:
/// - Only `http` or `https` schemes (file:// and others are rejected outright)
/// - Private IP ranges and link-local addresses are warned but ALLOWED when scheme is https
/// - Plain `http://` to non-localhost hosts emits a warning but is ALLOWED (developer tool)
/// - `http://localhost` and `http://127.0.0.1` are always allowed silently
fn validate_llm_url(base_url: &str) -> Result<(), String> {
    let parsed =
        url::Url::parse(base_url).map_err(|e| format!("Invalid URL '{}': {}", base_url, e))?;

    let scheme = parsed.scheme();

    // Reject file:// and any non-http/https scheme entirely
    if scheme == "file" {
        return Err("file:// URLs are not permitted for LLM API access".to_string());
    }
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "Unsupported URL scheme '{}'. Only http and https are allowed.",
            scheme
        ));
    }

    let host = parsed.host_str().unwrap_or("");

    // Determine if the host is localhost or the loopback address
    let is_local = host == "localhost" || host == "127.0.0.1";

    // Classify private / link-local ranges
    let is_private = {
        // Attempt to parse as an IPv4 address for range checks
        if let Ok(addr) = host.parse::<std::net::Ipv4Addr>() {
            let octets = addr.octets();
            // 10.0.0.0/8
            let is_10 = octets[0] == 10;
            // 172.16.0.0/12
            let is_172 = octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31;
            // 192.168.0.0/16
            let is_192 = octets[0] == 192 && octets[1] == 168;
            // 169.254.0.0/16 (link-local)
            let is_link_local = octets[0] == 169 && octets[1] == 254;
            is_10 || is_172 || is_192 || is_link_local
        } else {
            false
        }
    };

    if scheme == "http" {
        if is_private {
            tracing::warn!(
                "Warning: Using unencrypted HTTP connection to private/link-local address {}. \
                 Consider using HTTPS for production servers.",
                host
            );
        } else if !is_local {
            tracing::warn!(
                "Warning: Using unencrypted HTTP connection to {}. \
                 Consider using HTTPS for production servers.",
                host
            );
        }
        // localhost / 127.0.0.1 over http is fine — no warning needed
    }

    Ok(())
}

/// Application paths response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPaths {
    pub data_directory: String,
    pub log_directory: String,
}

/// System information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system (e.g., "linux", "windows", "macos")
    pub os: String,
    /// OS family (e.g., "unix", "windows")
    pub family: String,
    /// OS architecture (e.g., "x86_64", "aarch64", "arm")
    pub arch: String,
    /// OS version
    pub version: String,
    /// System locale
    pub locale: String,
}

/// Get the application data and log directories using Tauri's native path APIs
#[tauri::command]
pub async fn get_app_paths(app_handle: tauri::AppHandle) -> Result<AppPaths, String> {
    // Use Tauri's native path APIs for platform-agnostic directory resolution
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let log_dir = app_handle
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to get app log directory: {}", e))?;

    // Ensure directories exist
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir)
            .map_err(|e| format!("Failed to create log directory: {}", e))?;
    }

    Ok(AppPaths {
        data_directory: data_dir.to_string_lossy().to_string(),
        log_directory: log_dir.to_string_lossy().to_string(),
    })
}

/// Fetch available models from LLM API (avoids CORS issues)
#[tauri::command]
pub async fn fetch_llm_models(base_url: String) -> Result<Value, String> {
    validate_llm_url(&base_url)?;
    let client = reqwest::Client::new();
    // Handle base URL properly - if it already ends with /v1, just add /models
    let base = base_url.trim_end_matches('/');
    let models_url = if base.ends_with("/v1") {
        format!("{}/models", base)
    } else {
        format!("{}/v1/models", base)
    };

    match client.get(&models_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("Failed to parse models response: {}", e)),
                }
            } else {
                Err(format!("Models API returned status: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to fetch models: {}", e)),
    }
}

/// Make LLM completion request through Tauri (avoids CORS issues)
#[tauri::command]
pub async fn llm_completion_request(
    base_url: String,
    api_key: String,
    model: String,
    messages: Vec<Value>,
    max_tokens: Option<i32>,
    temperature: Option<f32>,
) -> Result<Value, String> {
    validate_llm_url(&base_url)?;
    if !api_key.is_empty() {
        tracing::debug!("API key received via IPC");
    }
    let client = reqwest::Client::new();
    // Handle baseUrl that may or may not already include /v1
    let base = base_url.trim_end_matches('/');
    let chat_url = if base.ends_with("/v1") {
        format!("{}/chat/completions", base)
    } else {
        format!("{}/v1/chat/completions", base)
    };

    let mut request_body = serde_json::json!({
        "model": model,
        "messages": messages
    });

    if let Some(max_tokens) = max_tokens {
        request_body["max_tokens"] = Value::Number(max_tokens.into());
    }

    if let Some(temperature) = temperature {
        request_body["temperature"] = Value::Number(
            serde_json::Number::from_f64(temperature as f64).unwrap_or(serde_json::Number::from(0)),
        );
    }

    let mut request_builder = client.post(&chat_url).json(&request_body);

    // Add API key if provided
    if !api_key.is_empty() {
        request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
    }

    match request_builder.send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("Failed to parse completion response: {}", e)),
                }
            } else {
                let status = response.status();
                match response.text().await {
                    Ok(error_text) => Err(format!("LLM API error ({}): {}", status, error_text)),
                    Err(_) => Err(format!("LLM API returned status: {}", status)),
                }
            }
        }
        Err(e) => Err(format!("Failed to complete LLM request: {}", e)),
    }
}

/// Get system information including OS details
/// Uses tauri-plugin-os to retrieve platform information
#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    // Get OS platform information using the os plugin
    let platform = tauri_plugin_os::platform().to_string();
    let arch = tauri_plugin_os::arch().to_string();
    let os_version = tauri_plugin_os::version().to_string();
    let family = tauri_plugin_os::family().to_string();
    let locale = tauri_plugin_os::locale();

    Ok(SystemInfo {
        os: platform,
        arch,
        version: os_version,
        family,
        locale: locale.unwrap_or_else(|| "unknown".to_string()),
    })
}

/// Issue #18 fix: Gracefully shutdown background monitoring tasks
/// Called by frontend before window close to prevent resource leaks
#[tauri::command]
pub async fn shutdown_background_tasks(
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    tracing::info!("Shutting down background tasks...");

    // Stop monitoring loop by aborting the task
    if let Some(handle) = state.monitoring_handle.lock().await.take() {
        handle.abort();
        tracing::info!("Monitoring loop stopped");
    } else {
        tracing::warn!("No monitoring handle found (already stopped or never started)");
    }

    // Give tasks time to clean up gracefully
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    tracing::info!("Background tasks shutdown complete");
    Ok(())
}
