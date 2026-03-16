//! AI-powered test generation
//!
//! Generates comprehensive test suites using LLM (Claude 3.7 Sonnet)
//! with automatic pattern detection and structured output.

use crate::error::{McpResult, McpStudioError};
use crate::llm_config::LLMConfigManager;
use crate::testing::analyzer::{ToolAnalysis, ToolInfo};
use crate::testing::{SchemaAnalyzer, TestDatabase};
use crate::types::{
    server_types::ServerInfo, GeneratedTest, GeneratedTestSuite, NewTest, NewTestSuite,
    TestCategory, TestKind,
};
use futures::future::join_all;
use std::sync::Arc;
use turbomcp_protocol::types::{
    ContentBlock, CreateMessageRequest, ModelHint, ModelPreferences, Role, SamplingMessage,
    TextContent,
};

/// AI-powered test generator
pub struct TestGenerator {
    llm: Arc<LLMConfigManager>,
    db: TestDatabase,
    mcp_manager: Arc<crate::mcp_client::McpClientManager>,
}

impl TestGenerator {
    /// Create a new test generator
    pub fn new(
        llm: Arc<LLMConfigManager>,
        db: TestDatabase,
        mcp_manager: Arc<crate::mcp_client::McpClientManager>,
    ) -> Self {
        Self {
            llm,
            db,
            mcp_manager,
        }
    }

    /// Generate a complete test suite for a server using per-tool parallel calls
    pub async fn generate_for_server(
        &self,
        server: &ServerInfo,
        provider_id: Option<String>,
        model_id: Option<String>,
    ) -> McpResult<String> {
        tracing::info!(
            "Starting per-tool parallel test generation for server: {:?}",
            server.id
        );

        // 1. Analyze server schema
        let analysis = SchemaAnalyzer::analyze_server(server);
        tracing::info!(
            "Schema analysis complete: {} patterns detected",
            analysis.patterns.len()
        );

        // 2. Extract actual tools from the server using MCP manager
        let tools = self.extract_actual_tools(server).await?;
        if tools.is_empty() {
            tracing::warn!("No tools found for server {}", server.id);
            return Err(McpStudioError::ConfigError(
                "Server has no tools to generate tests for".to_string(),
            ));
        }

        tracing::info!("Found {} tools for parallel test generation", tools.len());

        // 3. Create per-tool generation tasks
        let mut tasks = Vec::new();
        for tool in tools {
            let tool_analysis = SchemaAnalyzer::analyze_tool(&tool);
            let task = self.generate_tests_for_tool(
                server,
                tool,
                tool_analysis,
                provider_id.clone(),
                model_id.clone(),
            );
            tasks.push(task);
        }

        // 4. Execute all tool generations in PARALLEL
        tracing::info!(
            "Executing {} parallel LLM calls for tool test generation",
            tasks.len()
        );
        let results = join_all(tasks).await;

        // 5. Aggregate results
        let mut all_tests = Vec::new();
        let mut failures = Vec::new();

        for (idx, result) in results.into_iter().enumerate() {
            match result {
                Ok(tool_tests) => {
                    tracing::info!(
                        "Tool {} test generation succeeded: {} tests generated",
                        idx,
                        tool_tests.len()
                    );
                    all_tests.extend(tool_tests);
                }
                Err(e) => {
                    tracing::error!("Tool {} test generation failed: {}", idx, e);
                    failures.push((idx, e));
                }
            }
        }

        if all_tests.is_empty() {
            return Err(McpStudioError::ConfigError(format!(
                "All tool test generations failed ({} failures)",
                failures.len()
            )));
        }

        if !failures.is_empty() {
            tracing::warn!(
                "Partial success: {} tests generated, {} tool(s) failed",
                all_tests.len(),
                failures.len()
            );
        }

        // 6. Create aggregated suite
        let aggregated_suite = GeneratedTestSuite {
            suite_name: format!("{} - AI Generated Tests", server.config.name),
            description: Some(format!(
                "AI-generated test suite with {} total tests",
                all_tests.len()
            )),
            tests: all_tests,
        };

        // 7. Validate and save
        let test_count = aggregated_suite.tests.len();
        tracing::info!("Saving {} aggregated tests to database", test_count);
        let suite_id = self
            .save_to_database(server, aggregated_suite, &analysis)
            .await?;

        tracing::info!(
            "Test suite saved with ID: {} (total {} tests, {} tool failures)",
            suite_id,
            test_count,
            failures.len()
        );

        Ok(suite_id)
    }

    /// Extract actual tools from the server using MCP manager
    async fn extract_actual_tools(&self, server: &ServerInfo) -> McpResult<Vec<ToolInfo>> {
        tracing::info!("Extracting actual tools from server: {}", server.id);

        // Use MCP manager to get actual tool list
        match self.mcp_manager.list_tools(server.id).await {
            Ok(tools) => {
                let tool_infos: Vec<ToolInfo> = tools
                    .into_iter()
                    .map(|tool| ToolInfo {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                    })
                    .collect();

                tracing::info!(
                    "Extracted {} actual tools from server: {}",
                    tool_infos.len(),
                    server.id
                );
                Ok(tool_infos)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to list tools from server {}: {}. Falling back to synthetic tool.",
                    server.id,
                    e
                );
                // Fallback: return a synthetic tool representing all tools
                Ok(vec![ToolInfo {
                    name: format!(
                        "{}_tools",
                        server.config.name.to_lowercase().replace(" ", "_")
                    ),
                    description: Some(format!("All tools for {}", server.config.name)),
                }])
            }
        }
    }

    /// Generate tests for a SINGLE tool
    async fn generate_tests_for_tool(
        &self,
        server: &ServerInfo,
        tool: ToolInfo,
        tool_analysis: ToolAnalysis,
        provider_id: Option<String>,
        model_id: Option<String>,
    ) -> McpResult<Vec<GeneratedTest>> {
        tracing::info!(
            "Generating tests for tool: {} (complexity: {}) using per-category calls",
            tool.name,
            tool_analysis.complexity_score
        );

        // Define test categories with max_tokens (let LLM decide test count based on complexity)
        let categories = vec![
            (TestCategory::HappyPath, 3000), // Happy path tests (up to ~4 tests)
            (TestCategory::EdgeCase, 2500),  // Edge case tests (up to ~3 tests)
            (TestCategory::Error, 2500),     // Error handling tests (up to ~3 tests)
            (TestCategory::Security, 2000),  // Security tests (up to ~2 tests)
        ];

        // Create parallel tasks for each category
        let mut tasks = Vec::new();
        for (category, max_tokens) in categories {
            let prompt =
                self.build_category_test_prompt(server, &tool, category.clone(), &tool_analysis);
            let task = self.call_llm_for_category_tests(
                prompt, // Pass owned String instead of reference
                tool.name.clone(),
                category, // Use the original category here
                max_tokens,
                provider_id.clone(),
                model_id.clone(),
            );
            tasks.push(task);
        }

        // Execute all category generations in parallel
        let category_count = tasks.len(); // Get length before moving tasks
        tracing::info!(
            "Executing {} parallel category calls for tool: {}",
            category_count,
            tool.name
        );
        let results = join_all(tasks).await;

        // Aggregate category results
        let mut all_tests = Vec::new();
        for result in results {
            match result {
                Ok(category_tests) => {
                    all_tests.extend(category_tests);
                }
                Err(e) => {
                    tracing::warn!("Category test generation failed (continuing): {}", e);
                    // Continue with other categories even if one fails
                }
            }
        }

        if all_tests.is_empty() {
            return Err(McpStudioError::ConfigError(format!(
                "Failed to generate any tests for tool: {}",
                tool.name
            )));
        }

        tracing::info!(
            "Generated {} tests for tool: {} across {} categories",
            all_tests.len(),
            tool.name,
            category_count
        );
        Ok(all_tests)
    }

    /// Build category-specific prompt for focused test generation
    fn build_category_test_prompt(
        &self,
        server: &ServerInfo,
        tool: &ToolInfo,
        category: TestCategory,
        tool_analysis: &ToolAnalysis,
    ) -> String {
        let category_name = match category {
            TestCategory::HappyPath => "happy_path",
            TestCategory::EdgeCase => "edge_case",
            TestCategory::Error => "error",
            TestCategory::Security => "security",
            TestCategory::Workflow => "workflow",
            TestCategory::Performance => "performance",
        };

        let category_description = match category {
            TestCategory::HappyPath => "normal scenarios with valid inputs that should succeed",
            TestCategory::EdgeCase => {
                "boundary conditions, special characters, empty values, optional parameters"
            }
            TestCategory::Error => {
                "invalid inputs, missing required fields, wrong types that should fail gracefully"
            }
            TestCategory::Security => "injection attempts, validation bypass, potential exploits",
            TestCategory::Workflow => "multi-step sequences",
            TestCategory::Performance => "speed and load testing",
        };

        let expected_status = match category {
            TestCategory::HappyPath => "success",
            TestCategory::EdgeCase => "success (but test edge behavior)",
            TestCategory::Error => "error",
            TestCategory::Security => "error (should reject malicious input)",
            TestCategory::Workflow => "success",
            TestCategory::Performance => "success",
        };

        // Determine test count guidance based on tool complexity
        let complexity_guidance = if tool_analysis.complexity_score < 30 {
            "simple tool - generate 1-2 focused tests"
        } else if tool_analysis.complexity_score < 60 {
            "moderate tool - generate 2-3 comprehensive tests"
        } else {
            "complex tool - generate 3-4 thorough tests covering various scenarios"
        };

        format!(
            r#"GENERATE TEST(S) FOR CATEGORY: {category}

TOOL COMPLEXITY: {complexity_score}/100 ({guidance})

OUTPUT ONLY VALID JSON. NO MARKDOWN. NO EXPLANATIONS.

EXACT JSON STRUCTURE:
{{
  "suite_name": "{tool_name}_{category}_tests",
  "description": "{category} tests for {tool_name}",
  "tests": [
    {{
      "name": "test_identifier",
      "description": "What this specific test validates",
      "category": "{category}",
      "complexity": "simple",
      "kind": {{
        "tool_call": {{
          "tool_name": "{tool_name}",
          "arguments": {{"param": "value"}}
        }}
      }},
      "test_data": {{}},
      "assertions": [
        {{"type": "status_equals", "expected": "{status}"}}
      ]
    }}
  ]
}}

SERVER: {server_name}
TOOL: {tool_name}
DESCRIPTION: {tool_description}

CATEGORY: {category}
FOCUS: {category_desc}

REQUIREMENTS:
1. Generate tests appropriate for tool complexity: {guidance}
2. Each test MUST have category: "{category}"
3. Expected outcome: {status}
4. Output ONLY the JSON object starting with {{ and ending with }}
5. Include realistic test data based on tool description
6. Quality over quantity - ensure each test is meaningful
7. NO markdown formatting, NO code blocks

START YOUR RESPONSE WITH {{ and END WITH }}"#,
            category = category_name,
            complexity_score = tool_analysis.complexity_score,
            guidance = complexity_guidance,
            tool_name = tool.name,
            server_name = server.config.name,
            tool_description = tool.description.as_deref().unwrap_or("No description"),
            category_desc = category_description,
            status = expected_status,
        )
    }

    /// Call LLM for a specific category's tests with controlled max_tokens
    async fn call_llm_for_category_tests(
        &self,
        prompt: String,
        tool_name: String,
        category: TestCategory,
        max_tokens: u32,
        provider_id: Option<String>,
        model_id: Option<String>,
    ) -> McpResult<Vec<GeneratedTest>> {
        let category_name = match category {
            TestCategory::HappyPath => "happy_path",
            TestCategory::EdgeCase => "edge_case",
            TestCategory::Error => "error",
            TestCategory::Security => "security",
            TestCategory::Workflow => "workflow",
            TestCategory::Performance => "performance",
        };

        tracing::info!(
            "Calling LLM for tool: {} category: {} (max_tokens: {})",
            tool_name,
            category_name,
            max_tokens
        );

        // Build request with controlled max_tokens
        let request = CreateMessageRequest {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: ContentBlock::Text(TextContent {
                    text: prompt.to_string(),
                    annotations: None,
                    meta: None,
                }),
                metadata: None,
            }],
            system_prompt: Some("You are a QA engineer generating focused test cases. Output ONLY valid JSON with no markdown formatting.".to_string()),
            max_tokens,  // Use category-specific limit
            temperature: Some(0.3),
            model_preferences: model_id.clone().map(|m| {
                tracing::info!("Creating model preferences with hint: {}", m);
                ModelPreferences {
                    hints: Some(vec![ModelHint::new(m)]),
                    cost_priority: None,
                    speed_priority: None,
                    intelligence_priority: None,
                }
            }),
            include_context: None,
            stop_sequences: Some(vec![
                "```".to_string(),
                "```json".to_string(),
            ]),
            // v3 new fields - not needed for test generation
            tools: None,
            tool_choice: None,
            task: None,
            _meta: None,
        };

        // Invoke LLM
        let response = self
            .llm
            .invoke_llm_directly(request, provider_id.clone())
            .await
            .map_err(|e| {
                let error_msg = format!(
                    "LLM invocation failed for tool {} category {}: {}",
                    tool_name, category_name, e
                );
                tracing::error!("{}", error_msg);
                McpStudioError::ConfigError(error_msg)
            })?;

        // Extract text content
        let response_text = match response.content {
            ContentBlock::Text(text_content) => text_content.text,
            _ => {
                return Err(McpStudioError::ConfigError(format!(
                    "Unexpected response format for tool {} category {}",
                    tool_name, category_name
                )));
            }
        };

        tracing::debug!(
            "LLM response for {} {}: {} chars",
            tool_name,
            category_name,
            response_text.len()
        );

        // Extract JSON from response (strip markdown if present)
        let json_text = if let Some(start) = response_text.find('{') {
            if let Some(end) = response_text.rfind('}') {
                &response_text[start..=end]
            } else {
                &response_text
            }
        } else {
            &response_text
        };

        // Parse response - with smaller max_tokens, truncation should not occur
        let generated_suite: GeneratedTestSuite = serde_json::from_str(json_text).map_err(|e| {
            tracing::error!(
                "Failed to parse LLM response for tool {} category {}: {}",
                tool_name,
                category_name,
                e
            );
            tracing::error!(
                "Response JSON (first 500 chars): {}",
                if json_text.len() > 500 {
                    &json_text[..500]
                } else {
                    json_text
                }
            );
            McpStudioError::ConfigError(format!(
                "Failed to parse test JSON for tool {} category {}: {}",
                tool_name, category_name, e
            ))
        })?;

        // Validate tests are for the correct tool and category
        for test in &generated_suite.tests {
            if let TestKind::ToolCall {
                tool_name: test_tool,
                ..
            } = &test.kind
            {
                if test_tool != &tool_name {
                    tracing::warn!(
                        "Test '{}' targets wrong tool '{}' (expected '{}')",
                        test.name,
                        test_tool,
                        tool_name
                    );
                }
            }

            if test.category != category {
                tracing::warn!(
                    "Test '{}' has wrong category '{:?}' (expected '{:?}')",
                    test.name,
                    test.category,
                    category
                );
            }
        }

        tracing::info!(
            "Generated {} tests for tool {} category {}",
            generated_suite.tests.len(),
            tool_name,
            category_name
        );
        Ok(generated_suite.tests)
    }

    // Legacy monolithic test generation methods removed in v3 migration:
    // - build_test_generation_prompt() - superseded by per-category prompts
    // - get_capabilities_summary() - no longer needed
    // - call_llm_for_tests() - superseded by call_llm_for_category_tests()
    // The new per-tool parallel approach provides better LLM response quality.

    /// Extract JSON from LLM response
    /// Handles multiple reasoning/thinking formats:
    /// - Qwen thinking models: <think>...</think>{json}
    /// - OpenAI oss models: reasoning field + content field
    /// - Standard models: direct JSON
    /// - Markdown: ```json...```
    #[allow(dead_code)] // May be useful for future extensibility
    fn extract_json_from_response(&self, text: &str) -> String {
        let mut working_text = text.to_string();

        // Step 1: Extract content after closing </think> tag (Qwen thinking models)
        // Format: <think>reasoning...</think>{json response}
        if let Some(think_end) = text.find("</think>") {
            let after_think = &text[think_end + 8..]; // 8 = len("</think>")
            tracing::debug!("Detected <think> tags, extracting content after </think>");
            working_text = after_think.trim().to_string();

            // If nothing after think tag, fall through to other strategies
            if working_text.is_empty() {
                tracing::warn!("Response contained only thinking without output");
                // Fall through to next steps
            }
        }

        // Step 2: Try to extract from markdown code blocks (```json...```)
        if working_text.trim().starts_with("```json") || working_text.trim().starts_with("```") {
            let lines: Vec<&str> = working_text.lines().collect();
            if let Some(start_idx) = lines.iter().position(|l| l.starts_with("```")) {
                let start = start_idx + 1;
                if start < lines.len() {
                    if let Some(end_offset) =
                        lines[start..].iter().position(|l| l.starts_with("```"))
                    {
                        let end = start + end_offset;
                        tracing::debug!("Extracted JSON from markdown code block");
                        return lines[start..end].join("\n");
                    }
                }
            }
        }

        // Step 3: Try to find JSON object by locating first { and last }
        // This handles direct JSON, JSON after thinking tags, and JSON mixed with text
        if let Some(json_start) = working_text.find('{') {
            if let Some(json_end) = working_text.rfind('}') {
                if json_end > json_start {
                    // Extract from first { to last }
                    let json_text = &working_text[json_start..=json_end];

                    // Validate it looks like JSON before returning
                    if json_text.starts_with('{') && json_text.ends_with('}') {
                        tracing::debug!("Extracted JSON object from text");
                        return json_text.to_string();
                    }
                }
            }
        }

        // Step 4: Last resort - return working_text as-is
        // (might be plain text, partial JSON, or reasoning without output)
        tracing::warn!("Could not extract well-formed JSON from response");
        tracing::debug!("Response text: {}", working_text);
        working_text
    }

    /// Validate generated tests for basic correctness
    #[allow(dead_code)] // May be useful for future validation
    fn validate_generated_tests(&self, suite: &GeneratedTestSuite) -> McpResult<()> {
        if suite.tests.is_empty() {
            return Err(McpStudioError::ConfigError(
                "Generated test suite has no tests".to_string(),
            ));
        }

        for test in &suite.tests {
            if test.name.is_empty() {
                return Err(McpStudioError::ConfigError(
                    "Generated test has empty name".to_string(),
                ));
            }
            if test.assertions.is_empty() {
                return Err(McpStudioError::ConfigError(format!(
                    "Test '{}' has no assertions",
                    test.name
                )));
            }
        }

        Ok(())
    }

    /// Save generated tests to database
    async fn save_to_database(
        &self,
        server: &ServerInfo,
        suite: GeneratedTestSuite,
        analysis: &crate::types::SchemaAnalysis,
    ) -> McpResult<String> {
        // Create suite
        let suite_id = self
            .db
            .create_suite(NewTestSuite {
                server_id: server.id.to_string(),
                name: suite.suite_name,
                description: suite.description,
                generated_at: Some(std::time::SystemTime::now()),
                schema_hash: Some(analysis.hash.clone()),
            })
            .await?;

        // Convert generated tests to database format
        let tests_to_save: Vec<NewTest> = suite
            .tests
            .into_iter()
            .map(|test| NewTest {
                suite_id: suite_id.clone(),
                name: test.name,
                description: test.description,
                kind: test.kind,
                test_data: test.test_data,
                assertions: test.assertions,
                category: test.category,
                complexity: test.complexity,
                auto_generated: true,
            })
            .collect();

        // Batch insert tests
        self.db.create_tests(tests_to_save).await?;

        Ok(suite_id)
    }
}

// Legacy constants removed in v3 migration:
// - TEST_SCHEMA - was used for monolithic test generation
// - TEST_GENERATION_SYSTEM_PROMPT - superseded by inline prompts in call_llm_for_category_tests
// - TOOL_TEST_GENERATION_SYSTEM_PROMPT - never used

#[cfg(test)]
mod parse_tests {
    use super::*;
    use crate::types::Assertion;

    #[test]
    fn test_assertion_json_format() {
        // Test that assertions deserialize with "type" field
        let assertion_json = r#"{"type": "status_equals", "expected": "success"}"#;
        let result: Result<Assertion, _> = serde_json::from_str(assertion_json);
        assert!(
            result.is_ok(),
            "Failed to deserialize assertion: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_test_kind_json_format() {
        // Test that TestKind deserializes correctly
        let kind_json =
            r#"{"tool_call": {"tool_name": "test_tool", "arguments": {"arg": "value"}}}"#;
        let result: Result<TestKind, _> = serde_json::from_str(kind_json);
        assert!(
            result.is_ok(),
            "Failed to deserialize TestKind: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_complete_test_deserialization() {
        // Test deserializing a complete test object
        let test_json = r#"{
            "name": "test_name",
            "description": "test description",
            "category": "happy_path",
            "complexity": "simple",
            "kind": {"tool_call": {"tool_name": "test_tool", "arguments": {"url": "test"}}},
            "test_data": {},
            "assertions": [{"type": "status_equals", "expected": "success"}]
        }"#;

        let result: Result<GeneratedTest, _> = serde_json::from_str(test_json);
        assert!(
            result.is_ok(),
            "Failed to deserialize GeneratedTest: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_complete_suite_deserialization() {
        // Test deserializing a complete test suite
        let suite_json = r#"{
            "suite_name": "test_suite",
            "description": "test description",
            "tests": [
                {
                    "name": "test1",
                    "description": "desc",
                    "category": "happy_path",
                    "complexity": "simple",
                    "kind": {"tool_call": {"tool_name": "test_tool", "arguments": {}}},
                    "test_data": {},
                    "assertions": [{"type": "status_equals", "expected": "success"}]
                }
            ]
        }"#;

        let result: Result<GeneratedTestSuite, _> = serde_json::from_str(suite_json);
        assert!(
            result.is_ok(),
            "Failed to deserialize GeneratedTestSuite: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_multiple_assertions() {
        // Test deserializing multiple assertions with different types
        let assertions_json = r#"[
            {"type": "status_equals", "expected": "success"},
            {"type": "content_contains", "substring": "test"},
            {"type": "response_time_under", "milliseconds": 1000}
        ]"#;

        let result: Result<Vec<Assertion>, _> = serde_json::from_str(assertions_json);
        assert!(
            result.is_ok(),
            "Failed to deserialize assertions: {:?}",
            result.err()
        );

        if let Ok(assertions) = result {
            assert_eq!(assertions.len(), 3);
        }
    }

    #[test]
    fn test_invalid_assertion_missing_type() {
        // Test that assertions without "type" field fail
        let assertion_json = r#"{"expected": "success"}"#;
        let result: Result<Assertion, _> = serde_json::from_str(assertion_json);
        assert!(
            result.is_err(),
            "Should have failed to deserialize assertion without type"
        );
    }

    #[test]
    fn test_error_message_on_missing_type() {
        // Verify the error message mentions missing "type" field
        let assertion_json = r#"{"expected": "success"}"#;
        let result: Result<Assertion, _> = serde_json::from_str(assertion_json);

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("type") || error_msg.contains("unknown"),
                "Error should mention missing type field: {}",
                error_msg
            );
        } else {
            panic!("Should have failed");
        }
    }
}
