use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use turbomcp_client::sampling::{LLMServerClient, ServerInfo};
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult, Role};

/// Boxed future type alias for sampling operations (inlined from turbomcp-client v3 private type)
type BoxSamplingFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>>;

// ── OpenAI-compatible Chat Completions request/response types ────────────────

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    model: String,
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

// ── Client ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct OpenAICompatibleClient {
    http: Client,
    base_url: String,
    api_key: String,
    model: String,
    name: String,
}

impl OpenAICompatibleClient {
    pub fn new(name: String, base_url: String, api_key: String, model: String) -> Self {
        let http = Client::new();
        Self {
            http,
            base_url,
            api_key,
            model,
            name,
        }
    }

    pub fn new_ollama(model: String) -> Self {
        Self::new(
            "Ollama".to_string(),
            "http://localhost:11434/v1".to_string(),
            "ollama".to_string(),
            model,
        )
    }

    pub fn new_lmstudio(model: String) -> Self {
        Self::new(
            "LM Studio".to_string(),
            "http://localhost:1234/v1".to_string(),
            "lm-studio".to_string(),
            model,
        )
    }
}

impl LLMServerClient for OpenAICompatibleClient {
    fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> BoxSamplingFuture<'_, CreateMessageResult> {
        let http = self.http.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        Box::pin(async move {
            let mut messages: Vec<ChatMessage> = Vec::new();

            // Add system prompt if provided
            if let Some(system_prompt) = request.system_prompt {
                messages.push(ChatMessage {
                    role: "system",
                    content: system_prompt,
                });
            }

            // Convert MCP messages to OpenAI-compatible messages
            for msg in request.messages {
                let msg_json = serde_json::to_value(&msg)?;
                let content = &msg_json["content"];

                let text = if content.is_string() {
                    content.as_str().unwrap_or("").to_string()
                } else if content.is_object() && content["type"] == "text" {
                    content["text"].as_str().unwrap_or("").to_string()
                } else if content.is_array() {
                    let mut full_text = String::new();
                    if let Some(blocks) = content.as_array() {
                        for block in blocks {
                            if block["type"] == "text" {
                                full_text.push_str(block["text"].as_str().unwrap_or(""));
                            }
                        }
                    }
                    full_text
                } else {
                    return Err("Unsupported content type".into());
                };

                let role = match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                messages.push(ChatMessage {
                    role,
                    content: text,
                });
            }

            let body = ChatCompletionRequest {
                model: model.clone(),
                messages,
                max_tokens: request.max_tokens,
                temperature: request.temperature.unwrap_or(0.7) as f32,
            };

            let url = format!("{base_url}/chat/completions");
            let resp = http
                .post(&url)
                .bearer_auth(&api_key)
                .json(&body)
                .send()
                .await?;

            let status = resp.status();
            if !status.is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                return Err(format!("API error {status}: {err_text}").into());
            }

            let completion: ChatCompletionResponse = resp.json().await?;

            let choice = completion
                .choices
                .first()
                .ok_or("No completion choices returned")?;

            let content = choice.message.content.clone().unwrap_or_default();

            let result_json = serde_json::json!({
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": content
                },
                "model": completion.model,
                "stopReason": choice.finish_reason.as_deref().unwrap_or("end_turn")
            });

            let result: CreateMessageResult = serde_json::from_value(result_json)?;
            Ok(result)
        })
    }

    fn get_server_info(&self) -> BoxSamplingFuture<'_, ServerInfo> {
        let name = self.name.clone();
        let model = self.model.clone();
        Box::pin(async move {
            Ok(ServerInfo {
                name,
                models: vec![model],
                capabilities: vec![],
            })
        })
    }
}
