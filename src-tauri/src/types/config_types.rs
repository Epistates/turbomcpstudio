//! Application configuration types
//! 
//! Includes security settings, model preferences, and global app settings

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Security configuration for MCP Studio
/// 
/// Default is development mode (permissive) for local testing.
/// Users can opt into production mode through settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfig {
    /// Security mode: development (permissive) or production (strict)
    pub mode: SecurityMode,
    
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    
    /// Session security configuration
    pub session_security: SessionSecurityConfig,
    
    /// Allowed origins for CORS (empty = allow all in dev mode)
    pub allowed_origins: Vec<String>,
    
    /// API keys for authentication (empty = no auth in dev mode)
    pub api_keys: Vec<String>,
}

/// Security mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SecurityMode {
    /// Development mode - permissive for local testing
    Development,
    /// Production mode - strict security enforcement
    Production,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    
    /// Maximum requests per window
    pub max_requests: u32,
    
    /// Time window in seconds
    pub window_seconds: u64,
}

/// Session security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSecurityConfig {
    /// Maximum sessions per IP address
    pub max_sessions_per_ip: u32,
    
    /// Enforce IP binding (sessions locked to IP)
    pub enforce_ip_binding: bool,
    
    /// Regenerate session IDs on auth
    pub regenerate_session_ids: bool,
}

/// Model preferences configuration for sampling
/// 
/// Uses MCP 2025-06-18 compliant priority system (0.0-1.0)
/// instead of deprecated Low/Medium/High tiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferencesConfig {
    /// Enable model preferences
    pub enabled: bool,
    
    /// Model hints (e.g., ["claude-3-5-sonnet", "sonnet"])
    pub hints: Vec<String>,
    
    /// Cost priority (0.0 = not important, 1.0 = most important)
    pub cost_priority: Option<f64>,
    
    /// Speed priority (0.0 = not important, 1.0 = most important)
    pub speed_priority: Option<f64>,
    
    /// Intelligence priority (0.0 = not important, 1.0 = most important)
    pub intelligence_priority: Option<f64>,
}

/// Global application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    /// Security configuration
    pub security: SecurityConfig,
    
    /// Model preferences for sampling
    pub model_preferences: ModelPreferencesConfig,
    
    /// Auto-start sampling on server connection
    pub auto_start_sampling: bool,
    
    /// Default temperature for sampling (0.0-2.0)
    pub default_temperature: f64,
    
    /// Default max tokens for sampling
    pub default_max_tokens: Option<u32>,
}

impl Default for SecurityConfig {
    /// Default security config - DEVELOPMENT MODE
    /// 
    /// Permissive settings for local development and testing.
    /// Users can opt into production mode through settings UI.
    fn default() -> Self {
        Self {
            mode: SecurityMode::Development,
            rate_limiting: RateLimitConfig::default(),
            session_security: SessionSecurityConfig::default(),
            allowed_origins: vec![],  // Empty = allow all in dev mode
            api_keys: vec![],         // Empty = no auth in dev mode
        }
    }
}

impl Default for RateLimitConfig {
    /// Default rate limiting - DEVELOPMENT MODE
    /// 
    /// Disabled by default for local development.
    fn default() -> Self {
        Self {
            enabled: false,          // Disabled in dev mode
            max_requests: 1000,      // High limit when enabled
            window_seconds: 60,      // 1 minute window
        }
    }
}

impl Default for SessionSecurityConfig {
    /// Default session security - DEVELOPMENT MODE
    /// 
    /// Permissive settings for local development.
    fn default() -> Self {
        Self {
            max_sessions_per_ip: 100,    // High limit for dev
            enforce_ip_binding: false,   // Not enforced in dev
            regenerate_session_ids: true, // Always good practice
        }
    }
}

impl Default for ModelPreferencesConfig {
    /// Default model preferences
    /// 
    /// Balanced priorities, favoring quality and cost efficiency.
    fn default() -> Self {
        Self {
            enabled: true,
            hints: vec![],
            cost_priority: Some(0.7),           // Prefer cost-effective
            speed_priority: Some(0.5),          // Balanced speed
            intelligence_priority: Some(0.8),   // Prefer intelligent models
        }
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            security: SecurityConfig::default(),
            model_preferences: ModelPreferencesConfig::default(),
            auto_start_sampling: false,
            default_temperature: 0.7,
            default_max_tokens: None,
        }
    }
}

impl SecurityConfig {
    /// Create production security configuration with explicit parameters
    /// 
    /// Use this when deploying to production environments.
    pub fn for_production(
        allowed_origins: Vec<String>,
        api_keys: Vec<String>,
        max_requests: u32,
        window_seconds: u64,
    ) -> Self {
        Self {
            mode: SecurityMode::Production,
            rate_limiting: RateLimitConfig {
                enabled: true,
                max_requests,
                window_seconds,
            },
            session_security: SessionSecurityConfig {
                max_sessions_per_ip: 5,      // Strict limit
                enforce_ip_binding: true,    // Enforce in production
                regenerate_session_ids: true,
            },
            allowed_origins,
            api_keys,
        }
    }
}

impl RateLimitConfig {
    /// Get Duration from window_seconds
    pub fn window_duration(&self) -> Duration {
        Duration::from_secs(self.window_seconds)
    }
}

impl ModelPreferencesConfig {
    /// Convert to TurboMCP ModelPreferences type
    pub fn to_model_preferences(&self) -> Option<turbomcp_protocol::types::ModelPreferences> {
        if !self.enabled {
            return None;
        }

        use turbomcp_protocol::types::{ModelHint, ModelPreferences};

        let hints = if self.hints.is_empty() {
            None
        } else {
            Some(
                self.hints
                    .iter()
                    .map(|name| ModelHint::new(name.clone()))
                    .collect(),
            )
        };

        Some(ModelPreferences {
            hints,
            cost_priority: self.cost_priority,
            speed_priority: self.speed_priority,
            intelligence_priority: self.intelligence_priority,
        })
    }
}
