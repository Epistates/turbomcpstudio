use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client as OpenAIClientSDK,
};
use std::future::Future;
use std::pin::Pin;
use turbomcp_client::sampling::{LLMServerClient, ServerInfo};
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult, Role};

/// Boxed future type alias for sampling operations (inlined from turbomcp-client v3 private type)
type BoxSamplingFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>>;

#[derive(Clone, Debug)]
pub struct OpenAIClient {
    sdk: OpenAIClientSDK<OpenAIConfig>,
    model: String,
}

impl OpenAIClient {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let sdk = OpenAIClientSDK::with_config(config);
        let model = model.unwrap_or_else(|| "gpt-4o".to_string());
        Self { sdk, model }
    }
}

impl LLMServerClient for OpenAIClient {
    fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> BoxSamplingFuture<'_, CreateMessageResult> {
        let sdk = self.sdk.clone();
        let model = self.model.clone();

        Box::pin(async move {
            let mut messages: Vec<ChatCompletionRequestMessage> = Vec::new();

            // Add system prompt if provided
            if let Some(system_prompt) = request.system_prompt {
                messages.push(
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(system_prompt)
                        .build()?
                        .into(),
                );
            }

            // Convert MCP messages to OpenAI messages
            for msg in request.messages {
                // Using JSON fallback to avoid complex type mismatch issues with SamplingContentBlock
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

                let openai_msg: ChatCompletionRequestMessage = match msg.role {
                    Role::User => {
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(text)
                            .build()?
                            .into()
                    }
                    Role::Assistant => {
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .content(text)
                            .build()?
                            .into()
                    }
                };
                messages.push(openai_msg);
            }

            let response = sdk
                .chat()
                .create(
                    CreateChatCompletionRequestArgs::default()
                        .model(model)
                        .messages(messages)
                        .max_tokens(request.max_tokens as u32)
                        .temperature(request.temperature.unwrap_or(0.7) as f32)
                        .build()?,
                )
                .await?;

            let choice = response
                .choices
                .first()
                .ok_or_else(|| "No completion choices returned")?;

            let content = choice
                .message
                .content
                .clone()
                .unwrap_or_else(|| "".to_string());

            // Convert to JSON and back to CreateMessageResult to bypass strict type construction
            let result_json = serde_json::json!({
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": content
                },
                "model": response.model,
                "stopReason": choice.finish_reason.as_ref().map(|r| format!("{:?}", r))
            });

            let result: CreateMessageResult = serde_json::from_value(result_json)?;
            Ok(result)
        })
    }

    fn get_server_info(&self) -> BoxSamplingFuture<'_, ServerInfo> {
        Box::pin(async move {
            Ok(ServerInfo {
                name: "OpenAI".to_string(),
                models: vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()],
                capabilities: vec![],
            })
        })
    }
}
