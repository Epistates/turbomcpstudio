use std::future::Future;
use std::pin::Pin;
use turbomcp_client::sampling::{LLMServerClient, ServerInfo};
use turbomcp_protocol::types::{CreateMessageRequest, CreateMessageResult};

/// Boxed future type alias for sampling operations (inlined from turbomcp-client v3 private type)
type BoxSamplingFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>>;

#[derive(Clone, Debug)]
pub struct AnthropicClient {
    pub api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl LLMServerClient for AnthropicClient {
    fn create_message(
        &self,
        _request: CreateMessageRequest,
    ) -> BoxSamplingFuture<'_, CreateMessageResult> {
        let _api_key = self.api_key.clone();
        Box::pin(async move {
            // Anthropic implementation
            Err("Anthropic provider not yet implemented".into())
        })
    }

    fn get_server_info(&self) -> BoxSamplingFuture<'_, ServerInfo> {
        Box::pin(async move {
            Ok(ServerInfo {
                name: "Anthropic".to_string(),
                models: vec!["claude-3-5-sonnet-20240620".to_string()],
                capabilities: vec![],
            })
        })
    }
}
