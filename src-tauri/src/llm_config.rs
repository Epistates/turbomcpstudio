use crate::error::McpStudioError;
use crate::types::{
    LLMConfiguration, LLMProviderStatus, ProviderUsageStats, SetAPIKeyRequest,
    UpdateLLMConfigRequest,
};
use anyhow::Result;
use keyring::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use turbomcp_client::sampling::DelegatingSamplingHandler;

/// LLM Configuration Manager with secure credential storage
#[derive(Clone)]
pub struct LLMConfigManager {
    /// Current configuration (without sensitive data)
    config: Arc<RwLock<LLMConfiguration>>,
    /// Cached sampling handlers by provider ID
    handlers: Arc<RwLock<HashMap<String, Arc<DelegatingSamplingHandler>>>>,
    /// Usage statistics tracking
    usage_stats: Arc<RwLock<HashMap<String, ProviderUsageStats>>>,
    /// Service name for keyring storage
    keyring_service: String,
}

impl Default for LLMConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LLMConfigManager {
    /// Create a new LLM configuration manager
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(LLMConfiguration::default())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
            keyring_service: "mcp-studio".to_string(),
        }
    }

    /// Get the current LLM configuration (without sensitive data)
    pub async fn get_config(&self) -> LLMConfiguration {
        self.config.read().await.clone()
    }

    /// Update LLM configuration for a provider
    pub async fn update_provider_config(
        &self,
        request: UpdateLLMConfigRequest,
    ) -> Result<(), McpStudioError> {
        let mut config = self.config.write().await;
        config
            .providers
            .insert(request.provider_id.clone(), request.config);

        // If this provider is now enabled and was the only one, make it active
        if config.active_provider.is_none() {
            if let Some(provider_config) = config.providers.get(&request.provider_id) {
                if provider_config.enabled {
                    config.active_provider = Some(request.provider_id.clone());
                }
            }
        }

        // Clear cached handler since config changed
        self.handlers.write().await.remove(&request.provider_id);

        info!(
            "Updated LLM provider configuration: {}",
            request.provider_id
        );
        Ok(())
    }

    /// Set API key for a provider (stored securely in keyring)
    pub async fn set_api_key(&self, request: SetAPIKeyRequest) -> Result<(), McpStudioError> {
        let key_id = format!("{}-api-key", request.provider_id);

        // Store in system keyring
        let entry = Entry::new(&self.keyring_service, &key_id)
            .map_err(|e| McpStudioError::Configuration(format!("Keyring error: {}", e)))?;

        entry.set_password(&request.api_key).map_err(|e| {
            McpStudioError::Configuration(format!("Failed to store API key: {}", e))
        })?;

        // Enable the provider now that it has an API key
        let mut config = self.config.write().await;
        if let Some(provider_config) = config.providers.get_mut(&request.provider_id) {
            provider_config.enabled = true;

            // If no active provider, make this one active
            if config.active_provider.is_none() {
                config.active_provider = Some(request.provider_id.clone());
            }
        }

        // Clear cached handler to force recreation with new key
        self.handlers.write().await.remove(&request.provider_id);

        info!("API key set for provider: {}", request.provider_id);
        Ok(())
    }

    /// Get API key for a provider from keyring
    async fn get_api_key(&self, provider_id: &str) -> Option<String> {
        debug!("🔑 get_api_key called for provider: {}", provider_id);

        // Check if the provider exists and is enabled before accessing keyring
        // This prevents unnecessary keyring access for non-existent providers
        let config = self.config.read().await;
        debug!("🔑 Checking if provider {} exists in config", provider_id);

        let provider_config = match config.providers.get(provider_id) {
            Some(config) => {
                debug!(
                    "🔑 Provider {} found, enabled: {}",
                    provider_id, config.enabled
                );
                config
            }
            None => {
                debug!(
                    "🔑 Provider {} not found in config, returning None",
                    provider_id
                );
                return None;
            }
        };

        if !provider_config.enabled {
            debug!("🔑 Provider {} is disabled, returning None", provider_id);
            return None;
        }

        // Skip keyring access for local providers - they don't need API keys
        match provider_config.provider_type {
            crate::types::LLMProviderType::Local => {
                debug!(
                    "🔑 Provider {} is local type, returning None (no API key needed)",
                    provider_id
                );
                return None;
            }
            _ => {
                debug!(
                    "🔑 Provider {} is cloud type, proceeding with keyring access",
                    provider_id
                );
            }
        }
        drop(config);

        debug!(
            "🔑 Provider {} is enabled, attempting keyring access",
            provider_id
        );
        let key_id = format!("{}-api-key", provider_id);

        // Only access keyring if we actually need to read a key
        // This prevents keyring prompts when no LLM providers are configured
        debug!("🔑 Creating keyring entry for key_id: {}", key_id);
        let entry = match Entry::new(&self.keyring_service, &key_id) {
            Ok(entry) => {
                debug!("🔑 Successfully created keyring entry for {}", key_id);
                entry
            }
            Err(e) => {
                // If keyring access fails, silently return None
                // This prevents system keyring prompts for MCP servers that don't need API keys
                warn!("🔑 Failed to create keyring entry for {}: {}", key_id, e);
                return None;
            }
        };

        debug!("🔑 Attempting to get password from keyring for {}", key_id);
        match entry.get_password() {
            Ok(password) => {
                debug!(
                    "🔑 Successfully retrieved password from keyring for {}",
                    key_id
                );
                Some(password)
            }
            Err(e) => {
                debug!(
                    "🔑 Failed to get password from keyring for {}: {}",
                    key_id, e
                );
                None
            }
        }
    }

    /// Remove API key for a provider
    pub async fn remove_api_key(&self, provider_id: &str) -> Result<(), McpStudioError> {
        let key_id = format!("{}-api-key", provider_id);
        let entry = Entry::new(&self.keyring_service, &key_id)
            .map_err(|e| McpStudioError::Configuration(format!("Keyring error: {}", e)))?;

        if let Err(e) = entry.delete_password() {
            warn!("Failed to delete API key for {}: {}", provider_id, e);
        }

        // Disable the provider
        let mut config = self.config.write().await;
        if let Some(provider_config) = config.providers.get_mut(provider_id) {
            provider_config.enabled = false;
        }

        // If this was the active provider, clear it
        if config.active_provider.as_ref() == Some(&provider_id.to_string()) {
            config.active_provider = None;

            // Find another enabled provider to make active
            for (id, provider_config) in &config.providers {
                if provider_config.enabled && self.get_api_key(id).await.is_some() {
                    config.active_provider = Some(id.clone());
                    break;
                }
            }
        }

        // Remove cached handler
        self.handlers.write().await.remove(provider_id);

        info!("Removed API key for provider: {}", provider_id);
        Ok(())
    }

    /// Set the active provider
    pub async fn set_active_provider(&self, provider_id: String) -> Result<(), McpStudioError> {
        let config = self.config.read().await;

        // Validate that the provider exists and is configured
        match config.providers.get(&provider_id) {
            Some(provider_config) if provider_config.enabled => {
                // Check if API key exists (only for cloud providers - local providers don't need API keys)
                match provider_config.provider_type {
                    crate::types::LLMProviderType::Local => {
                        // Local providers don't need API keys, so they're always considered "configured"
                    }
                    _ => {
                        // Cloud providers need API keys
                        if self.get_api_key(&provider_id).await.is_none() {
                            return Err(McpStudioError::Configuration(format!(
                                "Provider {} is not configured with API key",
                                provider_id
                            )));
                        }
                    }
                }
            }
            Some(_) => {
                return Err(McpStudioError::Configuration(format!(
                    "Provider {} is not enabled",
                    provider_id
                )));
            }
            None => {
                return Err(McpStudioError::Configuration(format!(
                    "Unknown provider: {}",
                    provider_id
                )));
            }
        }

        drop(config);
        let mut config = self.config.write().await;
        config.active_provider = Some(provider_id.clone());

        info!("Set active LLM provider: {}", provider_id);
        Ok(())
    }

    /// Get a sampling handler for the active provider
    pub async fn get_active_sampling_handler(&self) -> Option<Arc<DelegatingSamplingHandler>> {
        let config = self.config.read().await;
        let active_provider_id = config.active_provider.clone()?;
        drop(config);

        self.get_sampling_handler(&active_provider_id).await
    }

    /// Get a sampling handler for a specific provider
    pub async fn get_sampling_handler(
        &self,
        provider_id: &str,
    ) -> Option<Arc<DelegatingSamplingHandler>> {
        // Check if we have a cached handler
        {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(provider_id) {
                return Some(handler.clone());
            }
        }

        // Create new handler
        let handler = self.create_sampling_handler(provider_id).await?;

        // Cache the handler
        let mut handlers = self.handlers.write().await;
        handlers.insert(provider_id.to_string(), handler.clone());

        Some(handler)
    }

    /// Create a new sampling handler for a provider
    async fn create_sampling_handler(
        &self,
        provider_id: &str,
    ) -> Option<Arc<DelegatingSamplingHandler>> {
        let config = self.config.read().await;
        let provider_config = config.providers.get(provider_id)?;

        if !provider_config.enabled {
            return None;
        }

        info!(
            "🚀 Creating production-grade sampling handler for provider: {}",
            provider_id
        );

        // Create DelegatingSamplingHandler based on provider type
        let handler = match provider_config.provider_type {
            crate::types::LLMProviderType::Local => {
                // Local providers (like Ollama) don't need API keys
                match self
                    .create_local_provider_handler(provider_id, provider_config)
                    .await
                {
                    Ok(handler) => {
                        info!("✅ Local sampling handler created for {}", provider_id);
                        handler
                    }
                    Err(e) => {
                        error!(
                            "❌ Failed to create local sampling handler for {}: {}",
                            provider_id, e
                        );
                        return None;
                    }
                }
            }
            _ => {
                // Cloud providers need API keys
                let api_key = match self.get_api_key(provider_id).await {
                    Some(key) => key,
                    None => {
                        warn!("🔑 No API key found for cloud provider: {}", provider_id);
                        return None;
                    }
                };

                match self
                    .create_cloud_provider_handler(provider_id, provider_config, &api_key)
                    .await
                {
                    Ok(handler) => {
                        info!("✅ Cloud sampling handler created for {}", provider_id);
                        handler
                    }
                    Err(e) => {
                        error!(
                            "❌ Failed to create cloud sampling handler for {}: {}",
                            provider_id, e
                        );
                        return None;
                    }
                }
            }
        };

        Some(Arc::new(handler))
    }

    /// Create sampling handler for local providers (Ollama, etc.)
    async fn create_local_provider_handler(
        &self,
        _provider_id: &str,
        _provider_config: &crate::types::LLMProviderConfig,
    ) -> Result<DelegatingSamplingHandler, McpStudioError> {
        use std::sync::Arc;
        use turbomcp_client::sampling::{AutoApprovingUserHandler, DelegatingSamplingHandler};

        // For now, create a simple auto-approving handler
        // TODO: Implement proper local LLM integration
        let user_handler = Arc::new(AutoApprovingUserHandler);
        let llm_clients = vec![]; // Empty for now - would connect to local LLM servers

        Ok(DelegatingSamplingHandler::new(llm_clients, user_handler))
    }

    /// Create sampling handler for cloud providers (OpenAI, Anthropic, etc.)
    async fn create_cloud_provider_handler(
        &self,
        provider_id: &str,
        _provider_config: &crate::types::LLMProviderConfig, // TODO: Use config for provider setup
        api_key: &str,
    ) -> Result<DelegatingSamplingHandler, McpStudioError> {
        use std::sync::Arc;
        use turbomcp_client::sampling::{AutoApprovingUserHandler, DelegatingSamplingHandler};

        // For now, create a simple auto-approving handler
        // TODO: Implement proper cloud LLM integration with provider_id: {} and api_key: {}
        let user_handler = Arc::new(AutoApprovingUserHandler);
        let llm_clients = vec![]; // Empty for now - would connect to cloud LLM servers
        info!(
            "Creating cloud handler for {} (key length: {})",
            provider_id,
            api_key.len()
        );

        Ok(DelegatingSamplingHandler::new(llm_clients, user_handler))
    }

    /// Get status of all providers
    pub async fn get_provider_statuses(&self) -> Vec<LLMProviderStatus> {
        debug!("🔍 get_provider_statuses called");
        let config = self.config.read().await;
        let usage_stats = self.usage_stats.read().await;
        let mut statuses = Vec::new();

        for (provider_id, provider_config) in &config.providers {
            debug!("🔍 Checking status for provider: {}", provider_id);

            // For local providers, consider them configured if enabled (no API key needed)
            // For cloud providers, check if API key exists in keyring
            let configured = match provider_config.provider_type {
                crate::types::LLMProviderType::Local => {
                    debug!("🔍 Provider {} is local, checking if enabled", provider_id);
                    provider_config.enabled
                }
                _ => {
                    debug!(
                        "🔍 Provider {} is cloud, checking API key in keyring",
                        provider_id
                    );
                    self.get_api_key(provider_id).await.is_some()
                }
            };

            let active = config.active_provider.as_ref() == Some(provider_id);
            debug!(
                "🔍 Provider {} - configured: {}, active: {}",
                provider_id, configured, active
            );

            statuses.push(LLMProviderStatus {
                provider_id: provider_id.clone(),
                display_name: provider_config.display_name.clone(),
                provider_type: provider_config.provider_type.clone(),
                enabled: provider_config.enabled,
                configured,
                active,
                available_models: provider_config.available_models.clone(),
                base_url: provider_config.base_url.clone(),
                last_error: None, // TODO: Track last errors
                usage_stats: usage_stats.get(provider_id).cloned(),
            });
        }

        debug!("🔍 Returning {} provider statuses", statuses.len());
        statuses
    }

    /// Initialize default providers if none exist
    pub async fn initialize_default_providers(&self) -> Result<(), McpStudioError> {
        let config = self.config.read().await;

        // If we already have providers, don't initialize defaults
        if !config.providers.is_empty() {
            return Ok(());
        }

        drop(config);

        // Load default configuration
        let mut config = self.config.write().await;
        *config = LLMConfiguration::default();

        info!("Initialized default LLM provider configurations");
        Ok(())
    }

    /// Validate that at least one provider is properly configured
    pub async fn validate_configuration(&self) -> Result<Vec<String>, McpStudioError> {
        debug!("✅ validate_configuration called");
        let config = self.config.read().await;
        let mut issues = Vec::new();

        // Check if any providers are enabled and configured
        let mut has_configured_provider = false;

        for (provider_id, provider_config) in &config.providers {
            debug!(
                "✅ Validating provider: {} (enabled: {})",
                provider_id, provider_config.enabled
            );
            if provider_config.enabled {
                if self.get_api_key(provider_id).await.is_some() {
                    debug!("✅ Provider {} has API key configured", provider_id);
                    has_configured_provider = true;
                } else {
                    debug!("✅ Provider {} is enabled but has no API key", provider_id);
                    issues.push(format!(
                        "Provider {} is enabled but has no API key",
                        provider_id
                    ));
                }
            }
        }

        if !has_configured_provider {
            debug!("✅ No configured providers found");
            issues.push("No LLM providers are properly configured".to_string());
        }

        if config.active_provider.is_none() && has_configured_provider {
            debug!("✅ No active provider selected despite having configured providers");
            issues.push("No active provider selected".to_string());
        }

        debug!("✅ Validation complete, {} issues found", issues.len());
        Ok(issues)
    }
}
