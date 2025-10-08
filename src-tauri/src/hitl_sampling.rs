/// Human-in-the-Loop Sampling Manager for MCP Studio
/// Implements the dual-mode sampling system from SAMPLING_DEVELOPER_FOCUSED.md
///
/// This module provides:
/// - Manual HITL mode for testing server behavior
/// - Real LLM mode for realistic testing
/// - Request/response interception for debugging
/// - Rich conversation context analysis
use crate::error::{McpResult, McpStudioError};
use crate::llm_config::LLMConfigManager;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use turbomcp_client::sampling::SamplingHandler;
use turbomcp_protocol::types::{
    Content, CreateMessageRequest, CreateMessageResult, Role, TextContent,
};
use uuid::Uuid;

/// Sampling mode configuration
#[allow(clippy::upper_case_acronyms)] // LLM is a well-known acronym
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum SamplingMode {
    /// Manual mode - all sampling requests go through human approval
    Manual {
        inspect_request: bool,
        edit_before_send: bool,
        mock_responses: bool,
    },
    /// LLM mode - automatic processing with real AI
    LLM {
        test_model_preferences: bool,
        compare_models: bool,
        validate_costs: bool,
    },
    /// Hybrid mode - smart routing based on rules
    Hybrid {
        auto_approval_rules: Vec<AutoApprovalRule>,
        fallback_to_human: bool,
    },
}

/// Rules for automatic approval in hybrid mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApprovalRule {
    pub name: String,
    pub condition: RuleCondition,
    pub action: RuleAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    TrustedServer(String),
    LowCostRequest(f64),        // Max cost threshold
    SafeContentPattern(String), // Regex pattern
    ModelPreference(String),    // Preferred model
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    AutoApprove,
    RequireReview,
    Reject(String), // Rejection reason
}

/// Pending sampling request awaiting human input
#[derive(Debug, Clone, Serialize)]
pub struct PendingSamplingRequest {
    pub id: String,
    pub server_id: String,
    pub server_name: String,
    pub request: CreateMessageRequest,
    pub estimated_cost: Option<f64>,
    pub estimated_tokens: Option<u32>,
    pub selected_model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub conversation_context: ConversationContext,
}

/// Rich conversation analysis for debugging
#[derive(Debug, Clone, Serialize)]
pub struct ConversationContext {
    pub thread_length: usize,
    pub total_tokens: Option<u32>,
    pub context_window_usage: Option<f32>, // 0.0 - 1.0
    pub relevant_messages: Vec<usize>,     // Indices of most relevant messages
    pub conversation_flow: Vec<ConversationTurn>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationTurn {
    pub role: Role,
    pub content_preview: String,
    pub token_count: Option<u32>,
    pub timestamp: Option<DateTime<Utc>>,
    pub relevance_score: Option<f32>,
}

/// Sampling response with rich metadata
#[derive(Debug, Clone, Serialize)]
pub struct SamplingResult {
    pub request_id: String,
    pub response: CreateMessageResult,
    pub model_used: String,
    pub token_usage: TokenUsage,
    pub cost_breakdown: CostAnalysis,
    pub response_time_ms: u64,
    pub stop_reason: String,
    pub processing_mode: ProcessingMode,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostAnalysis {
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum ProcessingMode {
    ManualApproval,
    AutomaticLLM,
    HybridRule(String), // Rule name that triggered
}

/// Events emitted by the HITL sampling system
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum HITLSamplingEvent {
    RequestPending {
        request: PendingSamplingRequest,
    },
    RequestApproved {
        request_id: String,
        approved_by: String,
    },
    RequestRejected {
        request_id: String,
        reason: String,
    },
    RequestCompleted {
        result: SamplingResult,
    },
    ModeChanged {
        old_mode: SamplingMode,
        new_mode: SamplingMode,
    },
    Error {
        request_id: Option<String>,
        error: String,
    },
}

/// Human-in-the-Loop Sampling Manager
pub struct HITLSamplingManager {
    /// Current sampling mode configuration
    mode: Arc<RwLock<SamplingMode>>,

    /// LLM configuration manager for real AI integration
    llm_config: Arc<LLMConfigManager>,

    /// Pending requests awaiting human approval
    pending_requests: Arc<DashMap<String, PendingSamplingRequest>>,

    /// Completed sampling results for analysis
    completed_requests: Arc<DashMap<String, SamplingResult>>,

    /// Event broadcaster for UI updates
    event_sender: broadcast::Sender<HITLSamplingEvent>,

    /// Request history for debugging and templates
    request_history: Arc<RwLock<Vec<PendingSamplingRequest>>>,
}

impl HITLSamplingManager {
    /// Create a new HITL sampling manager
    pub fn new(
        llm_config: Arc<LLMConfigManager>,
    ) -> (Self, broadcast::Receiver<HITLSamplingEvent>) {
        let (event_sender, event_receiver) = broadcast::channel(1000);

        let manager = Self {
            mode: Arc::new(RwLock::new(SamplingMode::Manual {
                inspect_request: true,
                edit_before_send: true,
                mock_responses: false,
            })),
            llm_config,
            pending_requests: Arc::new(DashMap::new()),
            completed_requests: Arc::new(DashMap::new()),
            event_sender,
            request_history: Arc::new(RwLock::new(Vec::new())),
        };

        info!("🎯 HITL Sampling Manager initialized with manual mode");
        (manager, event_receiver)
    }

    /// Set the sampling mode
    pub async fn set_mode(&self, new_mode: SamplingMode) -> McpResult<()> {
        let old_mode = {
            let mut mode = self.mode.write().await;
            let old = mode.clone();
            *mode = new_mode.clone();
            old
        };

        info!("🔄 Sampling mode changed: {:?} -> {:?}", old_mode, new_mode);

        let _ = self
            .event_sender
            .send(HITLSamplingEvent::ModeChanged { old_mode, new_mode });

        Ok(())
    }

    /// Get the current sampling mode
    pub async fn get_mode(&self) -> SamplingMode {
        self.mode.read().await.clone()
    }

    /// Process a sampling request with the current mode
    pub async fn process_sampling_request(
        &self,
        server_id: String,
        server_name: String,
        request: CreateMessageRequest,
    ) -> McpResult<SamplingResult> {
        let request_id = Uuid::new_v4().to_string();

        debug!(
            "🎯 Processing sampling request {} from server {}",
            request_id, server_name
        );

        // Analyze conversation context
        let conversation_context = self.analyze_conversation_context(&request).await?;

        // Estimate cost and tokens
        let (estimated_cost, estimated_tokens, selected_model) =
            self.estimate_request_cost(&request).await?;

        let pending_request = PendingSamplingRequest {
            id: request_id.clone(),
            server_id: server_id.clone(),
            server_name: server_name.clone(),
            request: request.clone(),
            estimated_cost,
            estimated_tokens,
            selected_model: selected_model.clone(),
            created_at: Utc::now(),
            conversation_context,
        };

        // Store in history
        self.request_history
            .write()
            .await
            .push(pending_request.clone());

        // Process based on current mode
        let mode = self.get_mode().await;
        match mode {
            SamplingMode::Manual { .. } => self.handle_manual_mode(pending_request).await,
            SamplingMode::LLM { .. } => self.handle_llm_mode(pending_request).await,
            SamplingMode::Hybrid {
                auto_approval_rules,
                fallback_to_human,
            } => {
                self.handle_hybrid_mode(pending_request, auto_approval_rules, fallback_to_human)
                    .await
            }
        }
    }

    /// Handle manual HITL mode - queue for human approval
    async fn handle_manual_mode(
        &self,
        pending_request: PendingSamplingRequest,
    ) -> McpResult<SamplingResult> {
        info!(
            "👤 Manual mode: Queuing request {} for human approval",
            pending_request.id
        );

        // Store pending request
        self.pending_requests
            .insert(pending_request.id.clone(), pending_request.clone());

        // Notify UI
        let _ = self.event_sender.send(HITLSamplingEvent::RequestPending {
            request: pending_request.clone(),
        });

        // For now, return a placeholder - in real implementation this would wait for human input
        let result = SamplingResult {
            request_id: pending_request.id.clone(),
            response: CreateMessageResult {
                role: Role::Assistant,
                content: Content::Text(TextContent {
                    text: "Request queued for human approval. Please check the HITL interface."
                        .to_string(),
                    annotations: None,
                    meta: None,
                }),
                model: "hitl-pending".to_string(),
                stop_reason: None,
                _meta: None,
            },
            model_used: "hitl-pending".to_string(),
            token_usage: TokenUsage {
                input_tokens: pending_request.estimated_tokens.unwrap_or(0),
                output_tokens: 0,
                total_tokens: pending_request.estimated_tokens.unwrap_or(0),
            },
            cost_breakdown: CostAnalysis {
                input_cost: 0.0,
                output_cost: 0.0,
                total_cost: pending_request.estimated_cost.unwrap_or(0.0),
                currency: "USD".to_string(),
            },
            response_time_ms: 0,
            stop_reason: "pending_human_approval".to_string(),
            processing_mode: ProcessingMode::ManualApproval,
        };

        Ok(result)
    }

    /// Handle automatic LLM mode - process with real AI
    async fn handle_llm_mode(
        &self,
        pending_request: PendingSamplingRequest,
    ) -> McpResult<SamplingResult> {
        info!(
            "🤖 LLM mode: Processing request {} with real AI",
            pending_request.id
        );

        let start_time = std::time::Instant::now();

        // Get active sampling handler
        let handler = self
            .llm_config
            .get_active_sampling_handler()
            .await
            .ok_or_else(|| {
                McpStudioError::Configuration("No active LLM provider configured".to_string())
            })?;

        // Process with real LLM
        let response = handler
            .handle_create_message(pending_request.request.clone())
            .await
            .map_err(|e| McpStudioError::TurboMcpError(format!("LLM sampling failed: {}", e)))?;

        let response_time = start_time.elapsed().as_millis() as u64;

        // Create rich result
        let result = SamplingResult {
            request_id: pending_request.id.clone(),
            response: response.clone(),
            model_used: pending_request
                .selected_model
                .unwrap_or("unknown".to_string()),
            token_usage: TokenUsage {
                input_tokens: pending_request.estimated_tokens.unwrap_or(0),
                output_tokens: self.estimate_output_tokens(&response).await,
                total_tokens: pending_request.estimated_tokens.unwrap_or(0),
            },
            cost_breakdown: CostAnalysis {
                input_cost: 0.0, // TODO: Calculate real costs
                output_cost: 0.0,
                total_cost: pending_request.estimated_cost.unwrap_or(0.0),
                currency: "USD".to_string(),
            },
            response_time_ms: response_time,
            stop_reason: "completed".to_string(),
            processing_mode: ProcessingMode::AutomaticLLM,
        };

        // Store completed result
        self.completed_requests
            .insert(pending_request.id.clone(), result.clone());

        // Notify UI
        let _ = self.event_sender.send(HITLSamplingEvent::RequestCompleted {
            result: result.clone(),
        });

        Ok(result)
    }

    /// Handle hybrid mode - smart routing with rules
    async fn handle_hybrid_mode(
        &self,
        pending_request: PendingSamplingRequest,
        auto_approval_rules: Vec<AutoApprovalRule>,
        fallback_to_human: bool,
    ) -> McpResult<SamplingResult> {
        info!(
            "🧠 Hybrid mode: Evaluating auto-approval rules for request {}",
            pending_request.id
        );

        // Evaluate auto-approval rules
        for rule in auto_approval_rules {
            if self.evaluate_rule(&rule, &pending_request).await? {
                match rule.action {
                    RuleAction::AutoApprove => {
                        info!(
                            "✅ Auto-approving request {} via rule: {}",
                            pending_request.id, rule.name
                        );
                        return self.handle_llm_mode(pending_request).await;
                    }
                    RuleAction::RequireReview => {
                        info!(
                            "👁️ Rule {} requires human review for request {}",
                            rule.name, pending_request.id
                        );
                        return self.handle_manual_mode(pending_request).await;
                    }
                    RuleAction::Reject(reason) => {
                        warn!(
                            "❌ Rejecting request {} via rule {}: {}",
                            pending_request.id, rule.name, reason
                        );
                        let _ = self.event_sender.send(HITLSamplingEvent::RequestRejected {
                            request_id: pending_request.id.clone(),
                            reason: reason.clone(),
                        });
                        return Err(McpStudioError::ProtocolError(format!(
                            "Request rejected by rule {}: {}",
                            rule.name, reason
                        )));
                    }
                }
            }
        }

        // No rules matched - use fallback
        if fallback_to_human {
            info!(
                "👤 No rules matched, falling back to human review for request {}",
                pending_request.id
            );
            self.handle_manual_mode(pending_request).await
        } else {
            info!(
                "🤖 No rules matched, falling back to automatic LLM for request {}",
                pending_request.id
            );
            self.handle_llm_mode(pending_request).await
        }
    }

    /// Evaluate an auto-approval rule
    async fn evaluate_rule(
        &self,
        rule: &AutoApprovalRule,
        request: &PendingSamplingRequest,
    ) -> McpResult<bool> {
        match &rule.condition {
            RuleCondition::TrustedServer(server_name) => Ok(request.server_name == *server_name),
            RuleCondition::LowCostRequest(max_cost) => {
                Ok(request.estimated_cost.unwrap_or(0.0) <= *max_cost)
            }
            RuleCondition::SafeContentPattern(_pattern) => {
                // TODO: Implement content pattern matching
                Ok(false)
            }
            RuleCondition::ModelPreference(model) => {
                Ok(request.selected_model.as_ref() == Some(model))
            }
        }
    }

    /// Analyze conversation context for debugging
    async fn analyze_conversation_context(
        &self,
        request: &CreateMessageRequest,
    ) -> McpResult<ConversationContext> {
        let thread_length = request.messages.len();

        // Convert messages for analysis
        let conversation_flow: Vec<ConversationTurn> = request
            .messages
            .iter()
            .map(|msg| {
                let content_preview = match &msg.content {
                    Content::Text(text) => {
                        let preview = if text.text.len() > 100 {
                            format!("{}...", &text.text[..97])
                        } else {
                            text.text.clone()
                        };
                        preview
                    }
                    // TODO: Handle other content types
                    _ => "[Non-text content]".to_string(),
                };

                ConversationTurn {
                    role: msg.role,
                    content_preview,
                    token_count: None, // TODO: Implement token counting
                    timestamp: None,
                    relevance_score: None, // TODO: Implement relevance scoring
                }
            })
            .collect();

        Ok(ConversationContext {
            thread_length,
            total_tokens: None,            // TODO: Implement total token counting
            context_window_usage: None,    // TODO: Calculate context window usage
            relevant_messages: Vec::new(), // TODO: Identify most relevant messages
            conversation_flow,
        })
    }

    /// Estimate request cost and tokens
    async fn estimate_request_cost(
        &self,
        _request: &CreateMessageRequest, // TODO: Use request for actual cost estimation
    ) -> McpResult<(Option<f64>, Option<u32>, Option<String>)> {
        // TODO: Implement real cost estimation based on model and message length
        let estimated_tokens = 1000; // Placeholder
        let estimated_cost = 0.001; // Placeholder
        let selected_model = Some("gpt-4o-mini".to_string()); // Placeholder

        Ok((Some(estimated_cost), Some(estimated_tokens), selected_model))
    }

    /// Estimate output tokens from response
    async fn estimate_output_tokens(&self, response: &CreateMessageResult) -> u32 {
        // TODO: Implement real token counting
        match &response.content {
            Content::Text(text) => (text.text.len() / 4) as u32, // Rough estimate
            _ => 0,
        }
    }

    /// Get pending requests for UI
    pub fn get_pending_requests(&self) -> Vec<PendingSamplingRequest> {
        self.pending_requests
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get completed results for analysis
    pub fn get_completed_requests(&self) -> Vec<SamplingResult> {
        self.completed_requests
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Approve a pending request (called from UI)
    pub async fn approve_request(
        &self,
        request_id: &str,
        approved_by: String,
        modified_request: Option<CreateMessageRequest>,
    ) -> McpResult<SamplingResult> {
        let pending_request = self
            .pending_requests
            .get(request_id)
            .ok_or_else(|| McpStudioError::Unknown(format!("Request not found: {}", request_id)))?
            .clone();

        // Use modified request if provided
        let final_request = modified_request.unwrap_or(pending_request.request.clone());

        info!("✅ Request {} approved by {}", request_id, approved_by);

        let _ = self.event_sender.send(HITLSamplingEvent::RequestApproved {
            request_id: request_id.to_string(),
            approved_by: approved_by.clone(),
        });

        // Remove from pending
        self.pending_requests.remove(request_id);

        // Process with LLM
        let mut updated_request = pending_request;
        updated_request.request = final_request;
        self.handle_llm_mode(updated_request).await
    }

    /// Reject a pending request (called from UI)
    pub async fn reject_request(&self, request_id: &str, reason: String) -> McpResult<()> {
        self.pending_requests
            .remove(request_id)
            .ok_or_else(|| McpStudioError::Unknown(format!("Request not found: {}", request_id)))?;

        info!("❌ Request {} rejected: {}", request_id, reason);

        let _ = self.event_sender.send(HITLSamplingEvent::RequestRejected {
            request_id: request_id.to_string(),
            reason,
        });

        Ok(())
    }
}
