//! AI-powered test generation
//!
//! Generates comprehensive test suites using LLM (Claude 3.7 Sonnet)
//! with automatic pattern detection and structured output.

use crate::error::{McpResult, McpStudioError};
use crate::llm_config::LLMConfigManager;
use crate::testing::{SchemaAnalyzer, TestDatabase};
use crate::testing::analyzer::{ToolAnalysis, ToolInfo};
use crate::types::{
    server_types::ServerInfo, Assertion, GeneratedTest, GeneratedTestSuite, NewTest, NewTestSuite,
    TestCategory, TestComplexity, TestKind,
};
use futures::future::join_all;
use std::sync::Arc;
use turbomcp_protocol::types::{Content, CreateMessageRequest, ModelHint, ModelPreferences, Role, SamplingMessage, TextContent};

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
        Self { llm, db, mcp_manager }
    }

    /// Generate a complete test suite for a server using per-tool parallel calls
    pub async fn generate_for_server(&self, server: &ServerInfo, provider_id: Option<String>, model_id: Option<String>) -> McpResult<String> {
        tracing::info!("Starting per-tool parallel test generation for server: {:?}", server.id);

        // 1. Analyze server schema
        let analysis = SchemaAnalyzer::analyze_server(server);
        tracing::info!("Schema analysis complete: {} patterns detected", analysis.patterns.len());

        // 2. Extract actual tools from the server using MCP manager
        let tools = self.extract_actual_tools(server).await?;
        if tools.is_empty() {
            tracing::warn!("No tools found for server {}", server.id);
            return Err(McpStudioError::ConfigError(
                "Server has no tools to generate tests for".to_string()
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
        tracing::info!("Executing {} parallel LLM calls for tool test generation", tasks.len());
        let results = join_all(tasks).await;

        // 5. Aggregate results
        let mut all_tests = Vec::new();
        let mut failures = Vec::new();

        for (idx, result) in results.into_iter().enumerate() {
            match result {
                Ok(tool_tests) => {
                    tracing::info!("Tool {} test generation succeeded: {} tests generated", idx, tool_tests.len());
                    all_tests.extend(tool_tests);
                }
                Err(e) => {
                    tracing::error!("Tool {} test generation failed: {}", idx, e);
                    failures.push((idx, e));
                }
            }
        }

        if all_tests.is_empty() {
            return Err(McpStudioError::ConfigError(
                format!("All tool test generations failed ({} failures)", failures.len())
            ));
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
            description: Some(format!("AI-generated test suite with {} total tests", all_tests.len())),
            tests: all_tests,
        };

        // 7. Validate and save
        let test_count = aggregated_suite.tests.len();
        tracing::info!("Saving {} aggregated tests to database", test_count);
        let suite_id = self.save_to_database(server, aggregated_suite, &analysis).await?;

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
                    server.id, e
                );
                // Fallback: return a synthetic tool representing all tools
                Ok(vec![ToolInfo {
                    name: format!("{}_tools", server.config.name.to_lowercase().replace(" ", "_")),
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
        tracing::info!("Generating tests for tool: {} (complexity: {})", tool.name, tool_analysis.complexity_score);

        // Build focused prompt for this tool only
        let prompt = self.build_tool_test_prompt(server, &tool);

        // Call LLM for this specific tool
        let tool_suite = self.call_llm_for_tool_tests(
            &prompt,
            &tool.name,
            provider_id,
            model_id,
        ).await?;

        tracing::info!("Generated {} tests for tool: {}", tool_suite.tests.len(), tool.name);
        Ok(tool_suite.tests)
    }

    /// Build focused prompt for a SINGLE tool
    fn build_tool_test_prompt(
        &self,
        server: &ServerInfo,
        tool: &ToolInfo,
    ) -> String {
        format!(
            r#"GENERATE TEST CASES FOR SINGLE TOOL. OUTPUT ONLY VALID JSON.

Expected JSON structure (MUST follow exactly):
{}

Server: {}
Tool to test: {}
Description: {}

Generate focused test cases for THIS TOOL ONLY by test category:
- Generate 1-2 "happy_path" tests for valid inputs and normal scenarios
- Generate 1-2 "edge_case" tests for boundary values, empty strings, special characters
- Generate 1-2 "error" tests for invalid params, wrong types, missing fields
- Generate 1 "security" test for injection/validation bypass attempts

Total: aim for 4-7 tests. You may generate more for complex tools, but prioritize quality over quantity.

For each test, provide: name, description, category, complexity, kind, test_data, assertions.

TEST CATEGORIES (MUST use exact names):
- "happy_path" (normal scenarios, valid inputs)
- "edge_case" (boundaries, special characters, optional params omitted)
- "error" (invalid params, missing fields, wrong types)
- "security" (injection, validation bypass attempts)

TEST COMPLEXITY:
- "simple" (straightforward test)
- "medium" (moderately complex)
- "complex" (advanced scenario)

ASSERTION FORMAT:
- {{"type": "status_equals", "expected": "success"|"error"}}
- {{"type": "content_contains", "substring": "..."}}
- {{"type": "response_time_under", "milliseconds": 1000}}

CRITICAL: Output ONLY the JSON object with "tests" array. No markdown. No explanations.
Start with {{ and end with }}.
"#,
            TEST_SCHEMA,
            server.config.name,
            tool.name,
            tool.description.as_deref().unwrap_or("No description"),
        )
    }

    /// Call LLM for a SINGLE tool's tests
    async fn call_llm_for_tool_tests(
        &self,
        prompt: &str,
        tool_name: &str,
        provider_id: Option<String>,
        model_id: Option<String>,
    ) -> McpResult<GeneratedTestSuite> {
        tracing::info!("Calling LLM for tool: {}", tool_name);

        // Get max_tokens from provider config
        let max_tokens = if let Some(ref prov_id) = provider_id {
            let config = self.llm.get_config().await;
            config
                .providers
                .get(prov_id)
                .map(|p| p.max_tokens)
                .unwrap_or(8000)
        } else {
            let config = self.llm.get_config().await;
            if let Some(active_id) = config.active_provider {
                config
                    .providers
                    .get(&active_id)
                    .map(|p| p.max_tokens)
                    .unwrap_or(8000)
            } else {
                8000
            }
        };

        tracing::info!("Using max_tokens: {} for tool: {}", max_tokens, tool_name);

        // Build request
        let request = CreateMessageRequest {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::Text(TextContent {
                    text: prompt.to_string(),
                    annotations: None,
                    meta: None,
                }),
                metadata: None,
            }],
            system_prompt: Some(TOOL_TEST_GENERATION_SYSTEM_PROMPT.to_string()),
            max_tokens,
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
            _meta: None,
        };

        // Invoke LLM
        let response = self
            .llm
            .invoke_llm_directly(request, provider_id.clone())
            .await
            .map_err(|e| {
                let error_msg = format!("LLM invocation failed for tool {}: {}", tool_name, e);
                tracing::error!("{}", error_msg);
                McpStudioError::ConfigError(error_msg)
            })?;

        // Extract text content
        let response_text = match response.content {
            Content::Text(text_content) => text_content.text,
            _ => {
                return Err(McpStudioError::ConfigError(format!(
                    "Unexpected response format for tool {}",
                    tool_name
                )))
            }
        };

        tracing::info!("LLM response length: {} chars for tool: {}", response_text.len(), tool_name);

        // Extract JSON from response
        let json_text = self.extract_json_from_response(&response_text);
        tracing::info!("Extracted JSON length: {} chars for tool: {}", json_text.len(), tool_name);

        // Log first and last 100 chars of extracted JSON for debugging
        let json_preview = if json_text.len() > 200 {
            format!("{}...{}", &json_text[..100], &json_text[json_text.len()-100..])
        } else {
            json_text.clone()
        };
        tracing::info!("Extracted JSON preview: {}", json_preview);

        // Parse response
        let generated_suite: GeneratedTestSuite = serde_json::from_str(&json_text)
            .map_err(|e| {
                tracing::error!("Failed to parse LLM response for tool {}: {}", tool_name, e);
                tracing::error!("Extracted JSON (first 500 chars): {}",
                    if json_text.len() > 500 { &json_text[..500] } else { &json_text });
                McpStudioError::ConfigError(format!(
                    "Failed to parse test JSON for tool {}: {}",
                    tool_name, e
                ))
            })?;

        // Validate tests are for the correct tool
        for test in &generated_suite.tests {
            if let TestKind::ToolCall { tool_name: test_tool, .. } = &test.kind {
                if test_tool != tool_name {
                    tracing::warn!(
                        "Test '{}' targets wrong tool '{}' (expected '{}')",
                        test.name, test_tool, tool_name
                    );
                }
            }
        }

        Ok(generated_suite)
    }

    /// Build the test generation prompt
    fn build_test_generation_prompt(
        &self,
        server: &ServerInfo,
        analysis: &crate::types::SchemaAnalysis,
    ) -> String {
        let server_name = &server.config.name;
        let server_desc = server.config.description.as_deref().unwrap_or("No description");

        // Get capabilities summary
        let (tools_summary, resources_summary, prompts_summary) = self.get_capabilities_summary(server);

        // Get patterns detected
        let patterns = analysis
            .patterns
            .iter()
            .map(|p| format!("{:?}", p))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"GENERATE TEST CASES. OUTPUT ONLY VALID JSON. NO OTHER TEXT.

Expected JSON structure (MUST follow exactly):
{}

Server to test:
Name: {}
Description: {}
Complexity Score: {}
Detected Patterns: {}

Capabilities:
Tools ({}):
{}

Resources ({}):
{}

Prompts ({}):
{}

Generate 15-25 comprehensive tests:
- Happy paths (40%): normal scenarios, valid inputs
- Edge cases (30%): boundaries, special chars, optional params omitted
- Error handling (20%): invalid params, missing fields, malformed input
- Security (10%): injection, auth bypass, validation

For each test, provide: name, description, category, complexity, kind, test_data, assertions.

CRITICAL: Output ONLY the JSON object. No thinking. No explanations. No markdown.
Start with {{ and end with }}. ONLY JSON.
"#,
            TEST_SCHEMA,
            server_name,
            server_desc,
            analysis.complexity.total_score,
            if patterns.is_empty() { "None detected".to_string() } else { patterns },
            analysis.complexity.tool_count,
            tools_summary,
            analysis.complexity.resource_count,
            resources_summary,
            analysis.complexity.prompt_count,
            prompts_summary,
        )
    }

    /// Get a summary of server capabilities
    fn get_capabilities_summary(&self, server: &ServerInfo) -> (String, String, String) {
        // TODO: Extract actual capabilities once we understand the structure
        // For now, return placeholder summaries

        let tools_summary = if let Some(caps) = &server.capabilities {
            // Try to extract tool information
            format!("Server has tools capability: {}", caps.tools.is_some())
        } else {
            "No capabilities information available".to_string()
        };

        let resources_summary = if let Some(caps) = &server.capabilities {
            format!("Server has resources capability: {}", caps.resources.is_some())
        } else {
            "No capabilities information available".to_string()
        };

        let prompts_summary = if let Some(caps) = &server.capabilities {
            format!("Server has prompts capability: {}", caps.prompts.is_some())
        } else {
            "No capabilities information available".to_string()
        };

        (tools_summary, resources_summary, prompts_summary)
    }

    /// Call LLM to generate tests
    async fn call_llm_for_tests(&self, prompt: &str, provider_id: Option<String>, model_id: Option<String>) -> McpResult<GeneratedTestSuite> {
        tracing::info!("=== Starting LLM call for test generation ===");
        tracing::info!("Provider ID: {:?}", provider_id);
        tracing::info!("Model ID: {:?}", model_id);
        tracing::info!("Prompt length: {} chars", prompt.len());

        // Get max_tokens from provider config or use default
        let max_tokens = if let Some(ref prov_id) = provider_id {
            let config = self.llm.get_config().await;
            config
                .providers
                .get(prov_id)
                .map(|p| p.max_tokens)
                .unwrap_or(8000)
        } else {
            // Fallback to default for active provider
            let config = self.llm.get_config().await;
            if let Some(active_id) = config.active_provider {
                config
                    .providers
                    .get(&active_id)
                    .map(|p| p.max_tokens)
                    .unwrap_or(8000)
            } else {
                8000
            }
        };

        tracing::info!("Using max_tokens: {} for provider: {:?}", max_tokens, provider_id);

        // Build request
        let request = CreateMessageRequest {
            messages: vec![SamplingMessage {
                role: Role::User,
                content: Content::Text(TextContent {
                    text: prompt.to_string(),
                    annotations: None,
                    meta: None,
                }),
                metadata: None,
            }],
            system_prompt: Some(TEST_GENERATION_SYSTEM_PROMPT.to_string()),
            max_tokens,  // Use configured value instead of hardcoded 8000
            temperature: Some(0.3),  // Lowered from 0.7 for more deterministic tests
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
            ]),  // Added stop sequences to prevent markdown after JSON
            _meta: None,
        };

        tracing::info!("Request created, invoking LLM directly...");
        tracing::info!("Messages count: {}, System prompt set: true", request.messages.len());

        // Use specified provider or fallback to active provider
        let response = self
            .llm
            .invoke_llm_directly(request, provider_id.clone())
            .await
            .map_err(|e| {
                let error_msg = format!("LLM invocation failed: {}", e);
                tracing::error!("{}", error_msg);
                tracing::error!("Provider was: {:?}", provider_id);
                McpStudioError::ConfigError(error_msg)
            })?;

        // Extract text content
        let response_text = match response.content {
            Content::Text(text_content) => text_content.text,
            _ => {
                return Err(McpStudioError::ConfigError(
                    "Unexpected response format from LLM".to_string(),
                ))
            }
        };

        // Parse JSON response
        // First try to extract JSON from markdown code blocks if present
        let json_text = self.extract_json_from_response(&response_text);

        tracing::info!("Extracted JSON length: {} chars", json_text.len());
        tracing::debug!("Extracted JSON (first 500 chars): {}",
            if json_text.len() > 500 { format!("{}...", &json_text[..500]) } else { json_text.clone() });
        tracing::info!("Full extracted JSON for parsing:\n{}", json_text);

        let generated_suite: GeneratedTestSuite = serde_json::from_str(&json_text)
            .map_err(|e| {
                tracing::error!("Failed to parse LLM response: {}", e);
                tracing::error!("Full JSON we tried to parse:\n{}", json_text);
                tracing::debug!("Original response text (first 1000 chars): {}",
                    if response_text.len() > 1000 { format!("{}...", &response_text[..1000]) } else { response_text.clone() });
                McpStudioError::ConfigError(format!("Failed to parse test suite JSON: {}", e))
            })?;

        // Validate generated tests
        self.validate_generated_tests(&generated_suite)?;

        Ok(generated_suite)
    }

    /// Extract JSON from LLM response
    /// Handles multiple reasoning/thinking formats:
    /// - Qwen thinking models: <think>...</think>{json}
    /// - OpenAI oss models: reasoning field + content field
    /// - Standard models: direct JSON
    /// - Markdown: ```json...```
    pub fn extract_json_from_response(&self, text: &str) -> String {
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
                    if let Some(end_offset) = lines[start..]
                        .iter()
                        .position(|l| l.starts_with("```"))
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

/// Expected JSON schema for test generation
const TEST_SCHEMA: &str = r#"{
  "suite_name": "Name of test suite",
  "description": "Brief description",
  "tests": [
    {
      "name": "test_name_here",
      "description": "What it validates",
      "category": "happy_path",
      "complexity": "simple",
      "kind": {"tool_call": {"tool_name": "name", "arguments": {}}},
      "test_data": {},
      "assertions": [
        {"type": "status_equals", "expected": "success"}
      ]
    }
  ]
}

VALID CATEGORIES:
- "happy_path" (normal scenarios, valid inputs)
- "edge_case" (boundaries, special characters, optional params)
- "error" (invalid params, missing fields, malformed input)
- "security" (injection, auth bypass, validation)
- "workflow" (multi-step operations)
- "performance" (performance/load testing)

VALID COMPLEXITIES:
- "simple"
- "medium"
- "complex"

VALID ASSERTION TYPES:
- {"type": "status_equals", "expected": "success"|"error"}
- {"type": "content_contains", "substring": "text"}
- {"type": "response_time_under", "milliseconds": 1000}
"#;

/// System prompt for test generation
/// Concise version that works with thinking and standard models
const TEST_GENERATION_SYSTEM_PROMPT: &str = r#"You are a QA engineer. Generate comprehensive test cases.

OUTPUT REQUIREMENT:
- Your response MUST be ONLY a valid JSON object
- Start with { and end with }
- No markdown, no explanations, no thinking process in the content
- If you are a reasoning model, use reasoning internally but output ONLY JSON

TEST CATEGORIES (MUST use exact names):
- "happy_path" (normal scenarios, valid inputs)
- "edge_case" (boundaries, special characters, optional params)
- "error" (invalid params, missing fields, malformed input)
- "security" (injection, validation bypass attempts)
- "workflow" (multi-step operations)
- "performance" (performance/load testing)

TEST COMPLEXITY (MUST use exact names):
- "simple" (straightforward test)
- "medium" (moderately complex)
- "complex" (advanced scenario)

ASSERTION FORMAT IN TESTS:
Use "type" field for each assertion:
- {"type": "status_equals", "expected": "success"|"error"}
- {"type": "content_contains", "substring": "..."}
- {"type": "response_time_under", "milliseconds": 1000}

GUIDELINES:
- Happy paths (40%): use category "happy_path"
- Edge cases (30%): use category "edge_case"
- Error handling (20%): use category "error"
- Security (10%): use category "security"
- Use actual server tool names and realistic parameters
"#;

/// System prompt for PER-TOOL test generation
/// Focused prompt for generating tests for a SINGLE tool at a time
const TOOL_TEST_GENERATION_SYSTEM_PROMPT: &str = r#"You are a QA engineer generating tests for a SINGLE MCP tool.

OUTPUT REQUIREMENT:
- Your response MUST be ONLY a valid JSON object with a "tests" array
- Start with { and end with }
- No markdown, no explanations, no thinking process in the content
- If you are a reasoning model, use reasoning internally but output ONLY JSON

FOCUS: Generate comprehensive tests for ONE SPECIFIC TOOL only.

TEST CATEGORIES (MUST use exact names):
- "happy_path" (normal scenarios, valid inputs)
- "edge_case" (boundaries, special characters, optional params omitted)
- "error" (invalid params, missing fields, wrong types)
- "security" (injection, validation bypass attempts)

TEST COMPLEXITY (MUST use exact names):
- "simple" (straightforward test)
- "medium" (moderately complex)
- "complex" (advanced scenario)

ASSERTION FORMAT IN TESTS:
- {"type": "status_equals", "expected": "success"|"error"}
- {"type": "content_contains", "substring": "..."}
- {"type": "response_time_under", "milliseconds": 1000}

GUIDELINES:
- Happy paths (40%): use category "happy_path"
- Edge cases (30%): use category "edge_case"
- Error handling (20%): use category "error"
- Security (10%): use category "security"
- Test realistic parameter combinations
- Cover required vs optional parameters
- Test error scenarios (missing required params, wrong types)
"#;

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn test_assertion_json_format() {
        // Test that assertions deserialize with "type" field
        let assertion_json = r#"{"type": "status_equals", "expected": "success"}"#;
        let result: Result<Assertion, _> = serde_json::from_str(assertion_json);
        assert!(result.is_ok(), "Failed to deserialize assertion: {:?}", result.err());
    }

    #[test]
    fn test_test_kind_json_format() {
        // Test that TestKind deserializes correctly
        let kind_json = r#"{"tool_call": {"tool_name": "test_tool", "arguments": {"arg": "value"}}}"#;
        let result: Result<TestKind, _> = serde_json::from_str(kind_json);
        assert!(result.is_ok(), "Failed to deserialize TestKind: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to deserialize GeneratedTest: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to deserialize GeneratedTestSuite: {:?}", result.err());
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
        assert!(result.is_ok(), "Failed to deserialize assertions: {:?}", result.err());

        if let Ok(assertions) = result {
            assert_eq!(assertions.len(), 3);
        }
    }

    #[test]
    fn test_invalid_assertion_missing_type() {
        // Test that assertions without "type" field fail
        let assertion_json = r#"{"expected": "success"}"#;
        let result: Result<Assertion, _> = serde_json::from_str(assertion_json);
        assert!(result.is_err(), "Should have failed to deserialize assertion without type");
    }

    #[test]
    fn test_error_message_on_missing_type() {
        // Verify the error message mentions missing "type" field
        let assertion_json = r#"{"expected": "success"}"#;
        let result: Result<Assertion, _> = serde_json::from_str(assertion_json);

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("type") || error_msg.contains("unknown"),
                   "Error should mention missing type field: {}", error_msg);
        } else {
            panic!("Should have failed");
        }
    }
}
