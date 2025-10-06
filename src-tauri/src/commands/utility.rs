//! Utility Commands
//!
//! Tauri commands for utility functions.
//! - Open URLs in browser
//! - Fetch LLM models
//! - Make LLM completion requests

use serde_json::Value;

/// Open a URL in the system's default browser
/// Note: Future feature - not yet registered
#[allow(dead_code)]
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    // Use system's default command to open URL
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", &url])
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    Ok(())
}

/// Fetch available models from LLM API (avoids CORS issues)
#[tauri::command]
pub async fn fetch_llm_models(base_url: String) -> Result<Value, String> {
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
