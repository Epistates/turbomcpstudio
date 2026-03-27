//! Proxy management commands for Tauri

use crate::proxy::{
    AuthConfig, BackendConfig, BenchmarkReport, BenchmarkSession, CallRecord, FrontendType,
    ProxyId, ProxyInfo, ProxyMetrics, ProxyStatus, ReportComparison, ServerSpec,
};
use tauri::State;

/// Create a new proxy configuration
#[tauri::command]
pub async fn create_proxy(
    name: String,
    description: Option<String>,
    backend_type: String,
    backend_config: serde_json::Value,
    frontend_type: String,
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    let frontend = match frontend_type.as_str() {
        "http" => FrontendType::Http,
        "websocket" => FrontendType::WebSocket,
        "tcp" => FrontendType::Tcp,
        _ => return Err(format!("Invalid frontend type: {}", frontend_type)),
    };

    let backend = match backend_type.as_str() {
        "stdio" => {
            let command = backend_config
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or("Missing stdio command")?
                .to_string();
            let args: Option<Vec<String>> = backend_config
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });

            BackendConfig::Stdio {
                command,
                args,
                env: None,
                working_dir: None,
            }
        }
        "http" => {
            let url = backend_config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing http url")?
                .to_string();

            BackendConfig::Http { url, headers: None }
        }
        "tcp" => {
            let host = backend_config
                .get("host")
                .and_then(|v| v.as_str())
                .ok_or("Missing tcp host")?
                .to_string();
            let port = backend_config
                .get("port")
                .and_then(|v| v.as_u64())
                .ok_or("Missing tcp port")? as u16;

            BackendConfig::Tcp { host, port }
        }
        "websocket" => {
            let url = backend_config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing websocket url")?
                .to_string();

            BackendConfig::WebSocket { url, headers: None }
        }
        _ => return Err(format!("Invalid backend type: {}", backend_type)),
    };

    let proxy_id = state
        .proxy_manager
        .create_proxy(name, description, backend, frontend, AuthConfig::None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(proxy_id.to_string())
}

/// Start a proxy
#[tauri::command]
pub async fn start_proxy(
    proxy_id: String,
    state: State<'_, crate::AppState>,
) -> Result<ProxyStatus, String> {
    let id = ProxyId(proxy_id);
    state
        .proxy_manager
        .start_proxy(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Stop a proxy
#[tauri::command]
pub async fn stop_proxy(proxy_id: String, state: State<'_, crate::AppState>) -> Result<(), String> {
    let id = ProxyId(proxy_id);
    state
        .proxy_manager
        .stop_proxy(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a proxy
#[tauri::command]
pub async fn delete_proxy(
    proxy_id: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    let id = ProxyId(proxy_id);
    state
        .proxy_manager
        .delete_proxy(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Get proxy status
#[tauri::command]
pub async fn get_proxy_status(
    proxy_id: String,
    state: State<'_, crate::AppState>,
) -> Result<ProxyStatus, String> {
    let id = ProxyId(proxy_id);
    state
        .proxy_manager
        .get_proxy_status(&id)
        .await
        .map_err(|e| e.to_string())
}

/// List all proxies
#[tauri::command]
pub async fn list_proxies(state: State<'_, crate::AppState>) -> Result<Vec<ProxyInfo>, String> {
    state
        .proxy_manager
        .list_proxies()
        .await
        .map_err(|e| e.to_string())
}

/// Get metrics for a proxy
#[tauri::command]
pub async fn get_proxy_metrics(
    proxy_id: String,
    state: State<'_, crate::AppState>,
) -> Result<ProxyMetrics, String> {
    let id = ProxyId(proxy_id);
    state
        .proxy_manager
        .get_proxy_metrics(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Introspect a backend to discover capabilities
#[tauri::command]
pub async fn introspect_backend(
    backend_type: String,
    backend_config: serde_json::Value,
    timeout_seconds: Option<u64>,
    state: State<'_, crate::AppState>,
) -> Result<ServerSpec, String> {
    let backend = match backend_type.as_str() {
        "stdio" => {
            let command = backend_config
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or("Missing stdio command")?
                .to_string();
            let args: Option<Vec<String>> = backend_config
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });

            BackendConfig::Stdio {
                command,
                args,
                env: None,
                working_dir: None,
            }
        }
        "http" => {
            let url = backend_config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing http url")?
                .to_string();

            BackendConfig::Http { url, headers: None }
        }
        _ => return Err(format!("Unsupported backend type: {}", backend_type)),
    };

    state
        .proxy_manager
        .introspect_backend(&backend, timeout_seconds)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Benchmark commands
// ---------------------------------------------------------------------------

/// Start a new benchmark session for a proxy.
///
/// Returns the session UUID callers must pass to subsequent benchmark commands.
#[tauri::command]
pub async fn start_benchmark_session(
    proxy_id: String,
    session_name: Option<String>,
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    state
        .proxy_manager
        .start_benchmark_session(&proxy_id, session_name)
        .await
        .map_err(|e| e.to_string())
}

/// Stop an active benchmark session and return its final report.
#[tauri::command]
pub async fn stop_benchmark_session(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<BenchmarkReport, String> {
    state
        .proxy_manager
        .stop_benchmark_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Return the raw call records for a session (active or stopped).
#[tauri::command]
pub async fn get_benchmark_records(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<Vec<CallRecord>, String> {
    state
        .proxy_manager
        .get_benchmark_records(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Generate and return a report from a session without stopping it.
#[tauri::command]
pub async fn get_benchmark_report(
    session_id: String,
    state: State<'_, crate::AppState>,
) -> Result<BenchmarkReport, String> {
    state
        .proxy_manager
        .get_benchmark_report(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// List all benchmark sessions (active and stopped), most recent first.
#[tauri::command]
pub async fn list_benchmark_sessions(
    state: State<'_, crate::AppState>,
) -> Result<Vec<BenchmarkSession>, String> {
    state
        .proxy_manager
        .list_benchmark_sessions()
        .await
        .map_err(|e| e.to_string())
}

/// Compare two benchmark reports and return per-backend deltas.
///
/// This command accepts the full report payloads directly so the frontend can
/// compare any two reports it has on hand without requiring them to still be
/// held in the manager's in-process state.
#[tauri::command]
pub async fn compare_benchmark_reports(
    report_a: BenchmarkReport,
    report_b: BenchmarkReport,
) -> Result<ReportComparison, String> {
    Ok(crate::proxy::compare_reports(&report_a, &report_b))
}
