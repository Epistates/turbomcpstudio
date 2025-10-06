<!--
  MCP Studio Sampling Workbench
  The ultimate MCP sampling testing interface - combines HITL + AI testing
  Enterprise-grade UX with pure DX focus for MCP server developers
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import Button from './ui/Button.svelte';
  import ErrorDiagnosis from './ui/ErrorDiagnosis.svelte';
  import { diagnoseError } from '$lib/utils/errorDiagnosis';
  import {
    MessageSquare,
    Bot,
    User,
    Settings,
    Play,
    Square,
    RefreshCw,
    AlertCircle,
    CheckCircle,
    Clock,
    Copy,
    Zap,
    Brain,
    Activity,
    Timer,
    Eye,
    EyeOff,
    Shield,
    FileText,
    Plus,
    Cpu,
    DollarSign,
    BarChart3,
    ChevronDown,
    ChevronRight,
    Workflow,
    TestTube,
    Pause,
    RotateCcw,
    Circle
  } from 'lucide-svelte';

  // ===============================================
  // TYPE DEFINITIONS
  // ===============================================

  interface SamplingMessage {
    role: 'user' | 'assistant' | 'system';
    content: {
      type: 'text' | 'image' | 'audio';
      text?: string;
      data?: string;
      mimeType?: string;
      annotations?: any;
      meta?: any;
    };
  }

  interface ModelPreferences {
    costPriority?: number; // 0-1
    speedPriority?: number; // 0-1
    intelligencePriority?: number; // 0-1
    hints?: { name: string }[];
  }

  interface SamplingResponse {
    role: 'assistant';
    content: { type: 'text'; text: string };
    model?: string;
    stopReason?: string;
    usage?: {
      inputTokens: number;
      outputTokens: number;
    };
  }

  interface SamplingRequest {
    id: string;
    serverId: string;
    serverName: string;
    messages: SamplingMessage[];
    modelPreferences?: ModelPreferences;
    systemPrompt?: string;
    includeContext?: 'none' | 'thisServer' | 'allServers';
    maxTokens?: number;
    temperature?: number;
    stopSequences?: string[];
    timestamp: string;
    status: 'pending' | 'approved' | 'rejected' | 'completed' | 'error';
    response?: SamplingResponse;
    error?: any; // Error information for diagnosis
    duration?: number;
    cost?: number;
  }

  interface LLMConfig {
    provider: 'openai' | 'anthropic' | 'local';
    apiKey: string;
    baseUrl?: string;
    organization?: string;
    model: string;
    defaultMaxTokens: number;
    defaultTemperature: number;
  }

  // ===============================================
  // STATE MANAGEMENT
  // ===============================================

  // Server and core state
  let servers: ServerInfo[] = $state([]);
  let selectedServerId: string | undefined = $state(undefined);
  let selectedServer: ServerInfo | undefined = $derived(
    servers.find(s => s.id === selectedServerId)
  );

  // Sampling requests and workflow
  let samplingRequests: SamplingRequest[] = $state([]);
  let selectedRequest: SamplingRequest | null = $state(null);
  let requestHistory: SamplingRequest[] = $state([]);

  // UI state
  let samplingMode: 'hitl' | 'ai' = $state('hitl');
  let showAdvancedOptions = $state(false);
  let loading = $state(false);
  let processing = $state(false);

  // LLM Configuration - Real-time provider status
  let llmProviders: any[] = $state([]);
  let activeProvider: any | null = $state(null);
  let samplingAvailable = $state(false);

  // LLM Configuration state - loaded from backend
  let llmConfig: LLMConfig = $state({
    provider: 'openai',
    apiKey: '',
    baseUrl: '',
    model: '',
    defaultMaxTokens: 500,
    defaultTemperature: 0.7
  });

  // Current active LLM provider from backend
  let activeLLMProvider: any = $state(null);

  // World-class cost estimation and analytics
  // estimatedCost moved to derived section for better performance
  let thinkingTokenCost = $state(0); // September 2025: Real thinking token costs
  let usageStats = $state({
    totalRequests: 0,
    successfulRequests: 0,
    failedRequests: 0,
    averageResponseTime: 0,
    totalCost: 0
  });

  // Test message composer
  let testMessage = $state('');
  let testSystemPrompt = $state('You are a helpful AI assistant working with MCP servers.');
  let testMaxTokens = $state(500);
  let testTemperature = $state(0.7);

  // 2024 Structured Output Features
  let useStructuredOutput = $state(false);
  let structuredOutputSchema = $state('{\n  "type": "object",\n  "properties": {\n    "response": {\n      "type": "string",\n      "description": "The main response"\n    }\n  },\n  "required": ["response"]\n}');
  let enableBatchMode = $state(false);
  let enableParallelFunctionCalls = $state(false);

  // September 2025 Cutting-Edge Features - Real API Parameters
  let reasoningEffort: "minimal" | "low" | "medium" | "high" = $state("medium"); // OpenAI GPT-5 reasoning_effort
  let verbosity: "low" | "medium" | "high" = $state("medium"); // OpenAI GPT-5 verbosity parameter
  let enableExtendedThinking = $state(false); // Claude 4.1 Opus extended thinking mode
  let enableComputerUse = $state(false); // Claude 4+ computer use beta
  let showThinkingProcess = $state(false); // Display thinking summaries
  let thinkingBudget = $state("medium"); // Claude thinking budget: low/medium/high
  let maxContextTokens = $state(200000); // Context window selection

  // ===============================================
  // DERIVED STATE
  // ===============================================

  const pendingRequests = $derived(
    samplingRequests.filter(req => req.status === 'pending')
  );

  const completedRequests = $derived(
    samplingRequests.filter(req =>
      req.status === 'completed' || req.status === 'error' || req.status === 'rejected'
    )
  );

  const hasLLMConfig = $derived(
    samplingAvailable && activeProvider && activeProvider.configured
  );

  // Convert expensive derived to debounced state to prevent keystroke cascade
  let canTestAI = $state(false);
  let canTestAITimeout: number;

  function updateCanTestAI() {
    clearTimeout(canTestAITimeout);
    canTestAITimeout = setTimeout(() => {
      canTestAI = !!(selectedServerId && hasLLMConfig && testMessage.trim().length > 0 && !creatingRequest);
    }, 100); // Very fast update for UI responsiveness
  }

  // Update canTestAI when relevant values change
  $effect(() => {
    updateCanTestAI();
  });

  // Track loading state for creating new requests
  let creatingRequest = $state(false);

  const activeProviderModel = $derived(
    activeProvider?.default_model || 'gpt-3.5-turbo'
  );

  // 2024 Feature Support Checking
  const supportsStructuredOutputs = $derived(
    activeProvider?.capabilities?.supports_structured_outputs || false
  );

  const supportsBatchProcessing = $derived(
    activeProvider?.capabilities?.supports_batch_processing || false
  );

  const supportsParallelFunctionCalling = $derived(
    activeProvider?.capabilities?.supports_parallel_function_calling || false
  );

  const isStructuredOutputModel = $derived(
    activeProvider?.capabilities?.structured_output_models?.includes(activeProviderModel) || false
  );

  // September 2025 Feature Support - Real API Specifications
  const supportsReasoningEffort = $derived(
    activeProvider?.provider_type === "openai" &&
    activeProviderModel?.includes('gpt-5') // GPT-5 models support reasoning_effort
  );

  const supportsExtendedThinking = $derived(
    activeProvider?.provider_type === "anthropic" &&
    activeProviderModel?.includes('claude-4') // Claude 4+ supports extended thinking
  );

  const supportsComputerUse = $derived(
    activeProvider?.capabilities?.supports_computer_use || false
  );

  const maxContextWindow = $derived(
    activeProvider?.capabilities?.max_context_tokens || 128000
  );

  const hasThinkingCost = $derived(
    activeProvider?.cost_config?.thinking_cost_per_1k !== undefined
  );

  // Dynamic context limits based on actual API specifications
  const contextTokenLimit = $derived(() => {
    if (activeProviderModel?.includes('gpt-5') && !activeProviderModel?.includes('nano')) {
      return 400000; // GPT-5: 272K input + 128K reasoning/output = 400K total
    } else if (activeProviderModel?.includes('claude-4')) {
      return 200000; // Claude 4 models: 200K context (1M beta for Sonnet 4)
    } else if (activeProviderModel?.includes('gpt-5-nano')) {
      return 128000; // GPT-5 Nano: 128K context
    }
    return maxContextWindow;
  });

  // Provider-specific feature detection
  const isOpenAIGPT5 = $derived(
    activeProvider?.provider_type === "openai" && activeProviderModel?.includes('gpt-5')
  );

  const isClaudeWithThinking = $derived(
    activeProvider?.provider_type === "anthropic" &&
    activeProviderModel?.includes('claude-4.1-opus') // Only 4.1 Opus has thinking tokens
  );

  // ===============================================
  // STORE SUBSCRIPTIONS
  // ===============================================

  // Use derived state instead of subscription effect for better performance
  const connectedServers = $derived(
    $serverStore.servers.filter((s: any) =>
      s.status?.toLowerCase() === 'connected' &&
      s.capabilities?.sampling
    )
  );

  // Only run when servers actually change
  $effect(() => {
    servers = connectedServers;

    if (selectedServerId !== $serverStore.selectedServerId) {
      selectedServerId = $serverStore.selectedServerId;
    }

    // Auto-select first connected server with sampling capability
    if (!$serverStore.selectedServerId && connectedServers.length > 0 && !selectedServerId) {
      serverStore.selectServer(connectedServers[0].id);
    }
  });

  // ===============================================
  // LLM CONFIGURATION & PROVIDER MANAGEMENT
  // ===============================================

  // Load LLM provider status from backend
  async function loadLLMProviders() {
    try {
      const [providersData, samplingStatus, llmConfigData] = await Promise.all([
        invoke('get_llm_provider_statuses'),
        invoke('is_sampling_available'),
        invoke('get_llm_config')
      ]);

      llmProviders = providersData as any[];
      samplingAvailable = samplingStatus as boolean;

      // Find active provider
      activeProvider = llmProviders.find(p => p.active) || null;
      activeLLMProvider = activeProvider;

      // Update LLM config from backend
      if (llmConfigData && typeof llmConfigData === 'object') {
        const backendConfig = llmConfigData as any;
        // Debug: Backend LLM config loaded

        if (backendConfig.active_provider && backendConfig.providers) {
          const activeProviderId = backendConfig.active_provider;
          const activeConfig = backendConfig.providers[activeProviderId];

          // Debug: Active provider ID loaded
          // Debug: Active provider config loaded

          if (activeConfig) {
            // Set the active LLM provider for display
            activeLLMProvider = {
              id: activeProviderId,
              display_name: activeConfig.display_name,
              provider_type: activeConfig.provider_type,
              enabled: activeConfig.enabled,
              default_model: activeConfig.default_model,
              base_url: activeConfig.base_url,
              available_models: activeConfig.available_models || []
            };

            llmConfig = {
              provider: (activeConfig.provider_type === 'local' ? 'local' :
                       activeConfig.provider_type === 'anthropic' ? 'anthropic' : 'openai') as 'openai' | 'anthropic' | 'local',
              apiKey: '', // Don't expose API keys in frontend
              baseUrl: activeConfig.base_url || 'http://localhost:1234/v1',
              model: activeConfig.default_model || 'gpt-3.5-turbo',
              defaultMaxTokens: 500,
              defaultTemperature: 0.7
            };

            // Debug: Final LLM config loaded
            // Debug: Final active LLM provider loaded

            // Use models from backend config instead of fetching again
            if (activeConfig.available_models && activeConfig.available_models.length > 0) {
              availableModels = activeConfig.available_models;
            } else {
              // Only fetch if no models in config
              availableModels = [];
            }
          }
        } else {
          // Debug: No active provider found
          // Reset to show unconfigured state
          activeLLMProvider = null;
          llmConfig = {
            provider: 'local' as const,
            apiKey: '',
            baseUrl: 'http://localhost:1234/v1',
            model: '',
            defaultMaxTokens: 500,
            defaultTemperature: 0.7
          };
        }
      }

      // Update usage stats if available
      if (activeProvider?.usage_stats) {
        usageStats = {
          totalRequests: activeProvider.usage_stats.total_requests || 0,
          successfulRequests: activeProvider.usage_stats.successful_requests || 0,
          failedRequests: activeProvider.usage_stats.failed_requests || 0,
          averageResponseTime: activeProvider.usage_stats.average_response_time_ms || 0,
          totalCost: activeProvider.usage_stats.total_cost || 0
        };
      }
    } catch (error) {
      console.error('Failed to load LLM providers:', error);
      uiStore.showError(`Failed to load LLM configuration: ${error}`);
    }
  }

  // Load available models from LLM API
  let availableModels: string[] = $state([]);

  async function loadAvailableModels() {
    if (!llmConfig.baseUrl) return;

    try {
      const data = await invoke<{ data?: any[] }>('fetch_llm_models', {
        baseUrl: llmConfig.baseUrl
      });

      if (data?.data) {
        availableModels = data.data.map((model: any) => model.id || model.name) || [];
      }
    } catch (error) {
      console.error('Failed to load models:', error);
      availableModels = [];
    }
  }

  // Estimate cost for a sampling request
  function estimateRequestCost(messages: any[], maxTokens: number = 500): number {
    if (!activeProvider?.cost_config) return 0;

    // Rough estimation: count input tokens (approximate)
    const inputText = messages.map(m => m.content?.text || '').join(' ');
    const estimatedInputTokens = Math.ceil(inputText.length / 4); // ~4 chars per token
    const estimatedOutputTokens = Math.min(maxTokens, 150); // Conservative estimate

    const inputCost = (estimatedInputTokens / 1000) * activeProvider.cost_config.input_cost_per_1k;
    const outputCost = (estimatedOutputTokens / 1000) * activeProvider.cost_config.output_cost_per_1k;

    // September 2025: Real API cost calculations
    let additionalCost = 0;

    // OpenAI GPT-5: Reasoning tokens count as output tokens
    if (isOpenAIGPT5 && reasoningEffort !== "minimal") {
      // Reasoning tokens are included in output token cost ($10/1M)
      // Higher reasoning effort = more reasoning tokens
      let reasoningMultiplier = 1.0;
      if (reasoningEffort === "low") reasoningMultiplier = 1.2;
      else if (reasoningEffort === "medium") reasoningMultiplier = 1.5;
      else if (reasoningEffort === "high") reasoningMultiplier = 2.0;

      // Adjust estimated output tokens to include reasoning
      const adjustedOutputTokens = estimatedOutputTokens * reasoningMultiplier;
      const adjustedOutputCost = (adjustedOutputTokens / 1000) * activeProvider.cost_config.output_cost_per_1k;
      additionalCost = adjustedOutputCost - outputCost;
    }

    // Claude 4.1 Opus: Separate thinking token pricing
    else if (isClaudeWithThinking && enableExtendedThinking && activeProvider.cost_config.thinking_cost_per_1k) {
      // Thinking budget affects thinking token usage
      let thinkingTokenMultiplier = 0.3; // Base thinking tokens
      if (thinkingBudget === "low") thinkingTokenMultiplier = 0.2;
      else if (thinkingBudget === "medium") thinkingTokenMultiplier = 0.4;
      else if (thinkingBudget === "high") thinkingTokenMultiplier = 0.6;

      const estimatedThinkingTokens = Math.ceil(estimatedInputTokens * thinkingTokenMultiplier);
      additionalCost = (estimatedThinkingTokens / 1000) * activeProvider.cost_config.thinking_cost_per_1k;
    }

    thinkingTokenCost = Number(additionalCost.toFixed(4));

    return Number((inputCost + outputCost + additionalCost).toFixed(4));
  }

  // Simple, non-blocking cost estimation - no memoization to avoid reactivity loops
  let estimatedCost = $state(0);
  let costCalculationTimeout: number;

  // Debounced cost calculation to prevent blocking on every keystroke
  function updateCostEstimation() {
    clearTimeout(costCalculationTimeout);
    costCalculationTimeout = setTimeout(() => {
      if (testMessage && activeProvider) {
        const messages = [{ content: { text: testMessage } }];
        estimatedCost = estimateRequestCost(messages, testMaxTokens);
      } else {
        estimatedCost = 0;
      }
    }, 300); // Quick update for cost display
  }

  // Trigger cost update on relevant changes - but debounced
  $effect(() => {
    updateCostEstimation();
  });

  // Validate LLM response for common issues
  function validateLLMResponse(result: any): { valid: boolean; reason?: string } {
    // Check if result exists
    if (!result) {
      return { valid: false, reason: 'Empty or null response from LLM' };
    }

    // Check for error status
    if (result.status === 'error') {
      return { valid: false, reason: result.message || 'LLM returned error status' };
    }

    // Check if content exists and is not empty
    const content = result.content || result.text || '';
    if (!content || content.trim().length === 0) {
      return {
        valid: false,
        reason: 'LLM returned empty response. This often happens when max_tokens is too low or the model hits a token limit. Try increasing max_tokens in the request.'
      };
    }

    // Check for problematic finish reasons
    if (result.stop_reason === 'length' && content.trim().length === 0) {
      return {
        valid: false,
        reason: 'LLM hit token limit before generating any content. Increase max_tokens or check your system prompt length.'
      };
    }

    return { valid: true };
  }

  // Intelligent retry with exponential backoff
  async function retryWithExponentialBackoff<T>(
    operation: () => Promise<T>,
    maxRetries: number = 3,
    initialDelayMs: number = 1000
  ): Promise<T> {
    let lastError: Error;

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await operation();
      } catch (error) {
        lastError = error as Error;

        // Don't retry on final attempt or for certain error types
        if (attempt === maxRetries || isNonRetryableError(error)) {
          throw error;
        }

        // Exponential backoff: 1s, 2s, 4s, 8s...
        const delay = initialDelayMs * Math.pow(2, attempt);
        const jitter = Math.random() * 0.1 * delay; // Add 10% jitter

        await new Promise(resolve => setTimeout(resolve, delay + jitter));
      }
    }

    throw lastError!;
  }

  function isNonRetryableError(error: any): boolean {
    const errorMessage = error?.toString().toLowerCase() || '';

    // Don't retry on authentication or configuration errors
    return errorMessage.includes('api key') ||
           errorMessage.includes('unauthorized') ||
           errorMessage.includes('invalid') ||
           errorMessage.includes('not found');
  }

  // Initialize LLM providers on mount
  onMount(() => {
    loadLLMProviders();

    // Refresh provider status every 30 seconds
    const interval = setInterval(loadLLMProviders, 30000);
    return () => clearInterval(interval);
  });

  // ===============================================
  // CORE FUNCTIONALITY
  // ===============================================

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
  }

  async function createTestSamplingRequest() {
    if (!selectedServerId || !testMessage.trim() || creatingRequest) return;

    creatingRequest = true;

    try {
      const testRequest: SamplingRequest = {
        id: crypto.randomUUID(),
        serverId: selectedServerId,
        serverName: selectedServer?.config.name || 'Test Server',
        messages: [
          {
            role: 'user',
            content: {
              type: 'text',
              text: testMessage.trim()
            }
          }
        ],
        modelPreferences: {
          intelligencePriority: 0.7,
          speedPriority: 0.6,
          costPriority: 0.4,
          hints: [{ name: llmConfig.model || activeProviderModel }]
        },
        systemPrompt: testSystemPrompt.trim() || undefined,
        includeContext: 'thisServer',
        maxTokens: testMaxTokens,
        temperature: testTemperature,
        timestamp: new Date().toISOString(),
        status: 'pending' as const
      };

      // Show immediate feedback
      samplingRequests = [testRequest, ...samplingRequests];
      selectedRequest = testRequest;

      // Clear the test form
      testMessage = '';

      if (samplingMode === 'ai' && hasLLMConfig) {
        uiStore.showSuccess(`Processing with ${llmConfig.model || activeProviderModel}...`);
      } else {
        uiStore.showSuccess('Sampling request created - awaiting human review');
      }

    } catch (error) {
      console.error('Failed to create sampling request:', error);
      uiStore.showError(`Failed to create request: ${error}`);
    } finally {
      creatingRequest = false;
    }
  }

  async function approveAndProcess(request: SamplingRequest, useAI: boolean = false) {
    if (!request) return;

    processing = true;
    request.status = 'approved';

    try {
      const startTime = Date.now();

      if (useAI && hasLLMConfig) {
        // Use AI processing via TurboMCP
        const createMessageRequest = {
          messages: request.messages,
          model_preferences: request.modelPreferences ? {
            cost_priority: request.modelPreferences.costPriority,
            speed_priority: request.modelPreferences.speedPriority,
            intelligence_priority: request.modelPreferences.intelligencePriority,
            hints: request.modelPreferences.hints
          } : undefined,
          system_prompt: request.systemPrompt,
          include_context: request.includeContext || 'none',
          temperature: request.temperature,
          max_tokens: request.maxTokens,
          stop_sequences: request.stopSequences,
          metadata: {
            user_id: 'mcp-studio-developer',
            session_id: crypto.randomUUID(),
            timestamp: new Date().toISOString()
          }
        };

        // Use the new runtime LLM configuration system with intelligent retry
        let result: SamplingResponse;
        try {
          result = await retryWithExponentialBackoff(async () => {
            return await invoke<SamplingResponse>('create_sampling_request', {
              serverId: request.serverId,
              messages: createMessageRequest.messages,
              maxTokens: createMessageRequest.max_tokens,
              temperature: createMessageRequest.temperature
            });
          });

          // Validate the response for common issues
          const isValidResponse = validateLLMResponse(result);
          if (!isValidResponse.valid) {
            throw new Error(isValidResponse.reason);
          }

          request.response = result;
          request.status = 'completed';

          // Update usage statistics
          usageStats.totalRequests++;
          usageStats.successfulRequests++;

          // Reload provider status to get updated usage stats
          loadLLMProviders();
        } catch (error) {
          throw error; // Re-throw to be caught by outer try-catch
        }
      } else {
        // HITL mode - use real HITL sampling system
        try {
          const hitlResult = await invoke<{ response: any; status: string }>('process_hitl_sampling_request', {
            serverId: request.serverId,
            serverName: request.serverName,
            request: {
              messages: request.messages,
              model_preferences: request.modelPreferences,
              system_prompt: request.systemPrompt,
              include_context: request.includeContext,
              max_tokens: request.maxTokens,
              temperature: request.temperature,
              stop_sequences: request.stopSequences
            }
          });

          request.response = hitlResult.response;
          request.status = hitlResult.status === 'completed' ? 'completed' : 'pending';

          // If pending, it means it's awaiting human approval
          if (request.status === 'pending') {
            uiStore.showInfo('Request submitted for human review - check the Sampling Debugger tab');
          }
        } catch (error) {
          console.error('HITL processing failed:', error);
          throw new Error(`HITL processing failed: ${error}`);
        }
      }

      request.duration = Date.now() - startTime;

      // Calculate cost from actual token usage
      if (request.response?.usage) {
        const inputCost = (request.response.usage.inputTokens / 1000) * 0.0015;
        const outputCost = (request.response.usage.outputTokens / 1000) * 0.002;
        request.cost = inputCost + outputCost;
      }

      samplingRequests = [...samplingRequests];

      // Move to history
      requestHistory = [request, ...requestHistory.slice(0, 19)]; // Keep last 20

      uiStore.showSuccess(
        useAI ? 'AI sampling completed successfully' : 'HITL request processed'
      );

    } catch (error) {
      request.status = 'error';
      request.error = error; // Store error for diagnosis
      request.response = {
        role: 'assistant',
        content: {
          type: 'text',
          text: `Error: ${error}`
        }
      };

      // Update failure statistics
      if (useAI) {
        usageStats.totalRequests++;
        usageStats.failedRequests++;

        // Reload provider status to get updated error stats
        loadLLMProviders();
      }

      samplingRequests = [...samplingRequests];
      uiStore.showError(`Sampling failed: ${error}`);
    } finally {
      processing = false;
    }
  }

  function rejectRequest(request: SamplingRequest) {
    request.status = 'rejected';
    samplingRequests = [...samplingRequests];
    uiStore.showInfo('Sampling request rejected');
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    uiStore.showSuccess('Copied to clipboard');
  }

  function formatMessageContent(content: any): string {
    if (typeof content === 'string') return content;
    if (content?.text) return content.text;
    return JSON.stringify(content, null, 2);
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'pending': return 'text-orange-700 bg-orange-100 border-orange-200';
      case 'approved': return 'text-blue-700 bg-blue-100 border-blue-200';
      case 'completed': return 'text-green-700 bg-green-100 border-green-200';
      case 'rejected': return 'text-red-700 bg-red-100 border-red-200';
      case 'error': return 'text-red-700 bg-red-100 border-red-200';
      default: return 'text-gray-700 bg-gray-100 border-gray-200';
    }
  }

  function getModelHintLabel(preferences?: ModelPreferences): string {
    if (!preferences?.hints?.[0]?.name) return 'Default model';
    return preferences.hints[0].name;
  }

  // ===============================================
  // LIFECYCLE
  // ===============================================

  onMount(async () => {
    // Load saved sampling state
    const savedSamplingState = localStorage.getItem('mcp-studio-sampling-state');
    if (savedSamplingState) {
      try {
        const parsed = JSON.parse(savedSamplingState);
        if (parsed.testMessage) testMessage = parsed.testMessage;
        if (parsed.testSystemPrompt) testSystemPrompt = parsed.testSystemPrompt;
        if (parsed.testMaxTokens) testMaxTokens = parsed.testMaxTokens;
        if (parsed.testTemperature) testTemperature = parsed.testTemperature;
        if (parsed.samplingRequests) samplingRequests = parsed.samplingRequests;
        if (parsed.selectedRequest) selectedRequest = parsed.selectedRequest;
      } catch (e) {
        console.warn('Failed to load saved sampling state:', e);
      }
    }

    // Initialize empty test message if nothing was saved
    if (!testMessage) testMessage = '';

    // Load LLM configuration from backend
    await loadLLMProviders();
  });

  // Consolidated localStorage saving with proper debouncing
  let saveTimeout: number;

  // Debounced save function to prevent excessive localStorage writes
  const debouncedSave = () => {
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      // Save LLM config
      localStorage.setItem('mcp-studio-llm-config', JSON.stringify(llmConfig));

      // Save sampling state
      const samplingState = {
        testMessage,
        testSystemPrompt,
        testMaxTokens,
        testTemperature,
        samplingRequests: samplingRequests.slice(0, 50), // Limit to last 50 requests
        selectedRequest
      };
      localStorage.setItem('mcp-studio-sampling-state', JSON.stringify(samplingState));
    }, 3000); // Further increased to 3 seconds to prevent excessive saves
  };

  // Minimal state saving - only when user stops typing
  $effect(() => {
    // Only save when we have meaningful content
    if (testMessage || testSystemPrompt || samplingRequests.length > 0) {
      debouncedSave();
    }
  });

  // Refresh LLM config when returning from Settings - optimized
  let previousView: string = '';
  $effect(() => {
    const currentView = $uiStore.currentView;
    if (previousView === 'settings' && currentView === 'sampling') {
      // Use requestIdleCallback for non-critical updates to prevent blocking
      if (typeof requestIdleCallback !== 'undefined') {
        requestIdleCallback(() => loadLLMProviders());
      } else {
        setTimeout(() => loadLLMProviders(), 10);
      }
    }
    previousView = currentView;
  });

  // World-class cleanup - prevent memory leaks
  onDestroy(() => {
    clearTimeout(saveTimeout);
    clearTimeout(costCalculationTimeout);
    clearTimeout(canTestAITimeout);

    // Clean up any pending operations
    if (processing) processing = false;
    if (creatingRequest) creatingRequest = false;
  });

</script>

<!-- =============================================== -->
<!-- MAIN COMPONENT MARKUP -->
<!-- =============================================== -->

<div class="h-full flex bg-gray-50 dark:bg-gray-900">

  <!-- LEFT PANEL: Request Queue & Controls -->
  <div class="w-96 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col">

    <!-- Header -->
    <div class="p-6 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-gradient-to-br from-mcp-primary-500 to-mcp-primary-600 flex items-center justify-center">
            <TestTube size={20} class="text-white" />
          </div>
          <div>
            <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
              Sampling Workbench
            </h2>
            <p class="text-sm text-gray-600 dark:text-gray-400">
              The Postman for MCP Sampling
            </p>
          </div>
        </div>
      </div>

      <!-- Mode Selection -->
      <div class="mb-4">
        <label for="sampling-mode" class="form-label">Mode</label>
        <div role="radiogroup" aria-labelledby="sampling-mode" class="grid grid-cols-2 gap-1 p-1 bg-gray-100 dark:bg-gray-700 rounded-lg">
          <button
            type="button"
            role="radio"
            aria-checked={samplingMode === 'hitl'}
            onclick={() => samplingMode = 'hitl'}
            class="px-3 py-2 text-sm font-medium rounded-md transition-all {samplingMode === 'hitl' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'}"
          >
            HITL Only
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={samplingMode === 'ai'}
            onclick={() => samplingMode = 'ai'}
            class="px-3 py-2 text-sm font-medium rounded-md transition-all {samplingMode === 'ai' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'}"
          >
            AI + HITL
          </button>
        </div>
        <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
          {#if samplingMode === 'hitl'}
            Human-in-the-loop approval only - manual testing and review
          {:else if samplingMode === 'ai'}
            AI processing with human oversight - real LLM responses with HITL fallback
          {/if}
        </p>
      </div>

      <!-- Server Selection -->
      {#if servers.length > 0}
        <div class="mb-4">
          <label for="workbench-server-select" class="form-label">Server</label>
          <select
            id="workbench-server-select"
            value={selectedServerId}
            onchange={(e) => selectServer((e.target as HTMLSelectElement).value)}
            class="form-input"
          >
            {#each servers as server}
              <option value={server.id}>
                {server.config.name || 'Unnamed Server'}
              </option>
            {/each}
          </select>
        </div>
      {/if}

      <!-- Quick Test Message Composer -->
      <div class="space-y-3">
        <label for="test-message-input" class="form-label flex items-center gap-2">
          <MessageSquare size={14} />
          Test Message
        </label>
        <textarea
          id="test-message-input"
          bind:value={testMessage}
          placeholder="Enter a message to test sampling... (⌘+Enter to send)"
          class="form-input h-20 resize-none transition-all duration-200 focus:ring-2 focus:ring-mcp-primary-500 focus:border-mcp-primary-500"
          onkeydown={(e) => {
            if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && !creatingRequest && selectedServerId && testMessage.trim()) {
              e.preventDefault();
              createTestSamplingRequest();
            }
          }}
        ></textarea>

        <div class="flex items-center gap-2">
          <Button
            variant="primary"
            size="sm"
            onclick={createTestSamplingRequest}
            disabled={!canTestAI}
            loading={creatingRequest}
            class="flex-1 transition-all duration-200 hover:scale-[1.02] active:scale-[0.98]"
          >
            <Plus size={14} class="transition-transform duration-200 {creatingRequest ? 'animate-spin' : ''}" />
            {creatingRequest ? 'Creating...' : 'Create Request'}
          </Button>
          <button
            type="button"
            onclick={() => showAdvancedOptions = !showAdvancedOptions}
            class="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-all duration-200 hover:scale-105 active:scale-95"
            title="Advanced options"
            aria-label="Toggle advanced options"
          >
            <Settings size={16} class="transition-transform duration-200 {showAdvancedOptions ? 'rotate-90' : ''}" />
          </button>
        </div>

        <div class="flex items-center justify-between text-xs text-gray-500">
          <span>
            <kbd class="kbd">⌘</kbd> + <kbd class="kbd">Enter</kbd> to create request
          </span>
          {#if estimatedCost > 0}
            <span class="text-green-600 font-medium animate-pulse">
              ~${estimatedCost.toFixed(4)} estimated
            </span>
          {/if}
        </div>
      </div>
    </div>

    <!-- HITL Options -->
    {#if showAdvancedOptions}
      <div class="p-4 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
        <div class="space-y-3">
          <div>
            <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2 flex items-center gap-2">
              <Settings size={14} class="text-blue-500" />
              HITL Testing Options
            </h4>
            <p class="text-xs text-gray-600 dark:text-gray-400 mb-3">
              Configure how requests are processed for testing your MCP server
            </p>
          </div>

          {#if samplingMode === 'ai'}
            <div>
              <label for="workbench-system-prompt" class="form-label text-xs">System Prompt (Optional)</label>
              <textarea
                id="workbench-system-prompt"
                bind:value={testSystemPrompt}
                class="form-input text-xs h-12 resize-none"
                placeholder="Additional context for the AI..."
              ></textarea>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label for="workbench-max-tokens" class="form-label text-xs">Max Tokens</label>
                <input
                  id="workbench-max-tokens"
                  type="number"
                  bind:value={testMaxTokens}
                  min="1"
                  max="4096"
                  class="form-input text-xs"
                />
              </div>
              <div>
                <label for="workbench-temperature" class="form-label text-xs">Temperature</label>
                <input
                  id="workbench-temperature"
                  type="number"
                  bind:value={testTemperature}
                  min="0"
                  max="2"
                  step="0.1"
                  class="form-input text-xs"
                />
              </div>
            </div>
          {:else}
            <div class="text-xs text-gray-600 dark:text-gray-400 italic">
              HITL Only mode - requests go directly to human review without AI processing
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- World-Class LLM Provider Status & Configuration -->
    {#if samplingMode === 'ai'}
      <div class="p-4 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between mb-3">
          <div class="flex items-center gap-2">
            <Bot size={16} class="text-mcp-primary-600" />
            <h3 class="font-medium text-gray-900 dark:text-white">AI Providers</h3>
            {#if samplingAvailable}
              <CheckCircle size={14} class="text-green-600" />
              <span class="text-xs text-green-600">Ready</span>
            {:else}
              <AlertCircle size={14} class="text-orange-500" />
              <span class="text-xs text-orange-600">Configure</span>
            {/if}
          </div>

          {#if estimatedCost > 0}
            <div class="flex items-center gap-2">
              <div class="flex items-center gap-1 text-xs text-gray-600">
                <DollarSign size={12} />
                <span>Est: ${estimatedCost}</span>
              </div>
              {#if thinkingTokenCost > 0}
                <div class="flex items-center gap-1 text-xs text-purple-600">
                  <Brain size={10} />
                  <span>${thinkingTokenCost} thinking</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>

        <!-- LLM Configuration -->
        <div class="space-y-2 mb-4">
          {#if activeLLMProvider}
            <div class="p-3 rounded-lg border border-green-200 bg-green-50 dark:bg-green-900/20 dark:border-green-800">
              <div class="flex items-center justify-between mb-3">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-medium">{activeLLMProvider.display_name || 'LLM Provider'}</span>
                  {#if availableModels.length > 0}
                    <CheckCircle size={12} class="text-green-600" />
                    <span class="text-xs text-green-600">Connected</span>
                  {:else if llmConfig.baseUrl}
                    <AlertCircle size={12} class="text-orange-500" />
                    <span class="text-xs text-orange-600">Loading...</span>
                  {:else}
                    <AlertCircle size={12} class="text-red-500" />
                    <span class="text-xs text-red-600">Not Configured</span>
                  {/if}
                </div>
                <button
                  onclick={() => uiStore.setView('settings')}
                  class="text-mcp-primary-600 hover:text-mcp-primary-700 font-medium text-xs"
                >
                  Configure in Settings →
                </button>
              </div>

              {#if llmConfig.baseUrl}
                <!-- Model Selection -->
                <div class="space-y-2">
                  <label class="block text-xs font-medium text-gray-700 dark:text-gray-300">
                    Model Selection
                  </label>
                  {#if availableModels.length > 0}
                    <select
                      bind:value={llmConfig.model}
                      class="w-full px-2 py-1.5 text-xs border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                    >
                      {#each availableModels as model}
                        <option value={model}>{model}</option>
                      {/each}
                    </select>
                  {:else}
                    <div class="flex items-center justify-between">
                      <span class="text-xs text-gray-600 dark:text-gray-400">{llmConfig.model || 'No model selected'}</span>
                      <button
                        onclick={loadAvailableModels}
                        class="text-xs text-mcp-primary-600 hover:text-mcp-primary-700"
                      >
                        Refresh Models
                      </button>
                    </div>
                  {/if}
                </div>
              {:else}
                <div class="text-xs text-gray-600 dark:text-gray-400">
                  Base URL not configured
                </div>
              {/if}
            </div>
          {:else}
            <div class="p-3 rounded-lg border border-orange-200 bg-orange-50">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <AlertCircle size={14} class="text-orange-500" />
                  <span class="text-sm font-medium text-gray-900">No AI Provider Configured</span>
                </div>
                <button
                  onclick={() => uiStore.setView('settings')}
                  class="btn-primary text-xs px-3 py-1"
                >
                  Configure in Settings
                </button>
              </div>
            </div>
          {/if}
        </div>


        <!-- Usage Statistics Summary -->
        {#if samplingAvailable && (usageStats.totalRequests > 0)}
          <div class="grid grid-cols-3 gap-3 p-3 bg-gray-50 rounded-lg">
            <div class="text-center">
              <div class="text-xs text-gray-500">Total</div>
              <div class="text-sm font-medium">{usageStats.totalRequests}</div>
            </div>
            <div class="text-center">
              <div class="text-xs text-gray-500">Success</div>
              <div class="text-sm font-medium text-green-600">
                {Math.round((usageStats.successfulRequests / Math.max(usageStats.totalRequests, 1)) * 100)}%
              </div>
            </div>
            <div class="text-center">
              <div class="text-xs text-gray-500">Cost</div>
              <div class="text-sm font-medium">${usageStats.totalCost.toFixed(3)}</div>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- World-Class Request Queue with Smooth Interactions -->
    <div class="flex-1 overflow-y-auto scroll-smooth">

      <!-- Pending Requests -->
      {#if pendingRequests.length > 0}
        <div class="p-4 border-b border-gray-200 dark:border-gray-700">
          <h3 class="text-sm font-medium text-gray-900 dark:text-white mb-3 flex items-center gap-2">
            <Clock size={14} class="text-orange-500" />
            Pending ({pendingRequests.length})
          </h3>

          <div class="space-y-2">
            {#each pendingRequests as request}
              <button
                onclick={() => selectedRequest = request}
                class="w-full p-3 text-left bg-orange-50 hover:bg-orange-100 border border-orange-200 rounded-lg transition-colors {selectedRequest?.id === request.id ? 'ring-2 ring-orange-400' : ''}"
              >
                <div class="flex items-center justify-between mb-1">
                  <span class="text-sm font-medium text-gray-900">
                    {request.serverName}
                  </span>
                  <span class="text-xs text-orange-700 bg-orange-100 px-2 py-1 rounded-full">
                    Pending
                  </span>
                </div>
                <p class="text-xs text-gray-600 line-clamp-2">
                  {formatMessageContent(request.messages.find(m => m.role === 'user')?.content)}
                </p>
                <div class="flex items-center justify-between mt-2 text-xs text-gray-500">
                  <span>{request.messages.length} messages</span>
                  <span>{new Date(request.timestamp).toLocaleTimeString()}</span>
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Completed Requests -->
      {#if completedRequests.length > 0}
        <div class="p-4">
          <h3 class="text-sm font-medium text-gray-900 dark:text-white mb-3 flex items-center gap-2">
            <CheckCircle size={14} class="text-green-600" />
            Recent ({completedRequests.length})
          </h3>

          <div class="space-y-2">
            {#each completedRequests.slice(0, 10) as request}
              <button
                onclick={() => selectedRequest = request}
                class="w-full p-3 text-left bg-gray-50 hover:bg-gray-100 border border-gray-200 rounded-lg transition-colors {selectedRequest?.id === request.id ? 'ring-2 ring-mcp-primary-400' : ''}"
              >
                <div class="flex items-center justify-between mb-1">
                  <span class="text-sm font-medium text-gray-900">
                    {request.serverName}
                  </span>
                  <span class="text-xs px-2 py-1 rounded-full border {getStatusColor(request.status)}">
                    {request.status}
                  </span>
                </div>
                <div class="flex items-center justify-between text-xs text-gray-500">
                  <span>
                    {#if request.duration}
                      {request.duration}ms
                    {/if}
                    {#if request.cost}
                      • ${request.cost.toFixed(4)}
                    {/if}
                  </span>
                  <span>{new Date(request.timestamp).toLocaleTimeString()}</span>
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Empty State -->
      {#if samplingRequests.length === 0}
        <div class="flex items-center justify-center h-48">
          <div class="text-center">
            <TestTube size={32} class="mx-auto text-gray-400 mb-3" />
            <h3 class="text-sm font-medium text-gray-900 dark:text-white">
              No Sampling Requests
            </h3>
            <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
              Create a test request to get started
            </p>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- RIGHT PANEL: Request Details & Processing -->
  <div class="flex-1 flex flex-col bg-white dark:bg-gray-800">

    {#if selectedRequest}
      <!-- Header -->
      <div class="p-6 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-start justify-between">
          <div>
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">
              Sampling Request
            </h3>
            <div class="flex items-center gap-4 mt-2 text-sm">
              <span class="px-3 py-1 rounded-full border text-xs font-medium {getStatusColor(selectedRequest.status)}">
                {selectedRequest.status.toUpperCase()}
              </span>
              <span class="text-gray-600 dark:text-gray-400">
                {selectedRequest.serverName}
              </span>
              <span class="text-gray-500 dark:text-gray-500">
                {new Date(selectedRequest.timestamp).toLocaleString()}
              </span>
            </div>
          </div>

          <!-- Action Buttons -->
          {#if selectedRequest.status === 'pending'}
            <div class="flex items-center gap-2">
              <Button
                variant="secondary"
                size="sm"
                onclick={() => rejectRequest(selectedRequest!)}
                disabled={processing}
              >
                <Square size={14} />
                Reject
              </Button>

              {#if samplingMode === 'hitl'}
                <Button
                  variant="primary"
                  size="sm"
                  onclick={() => approveAndProcess(selectedRequest!, false)}
                  disabled={processing}
                >
                  <User size={14} />
                  HITL Approve
                </Button>
              {/if}

              {#if samplingMode === 'ai' && hasLLMConfig}
                <Button
                  variant="secondary"
                  size="sm"
                  onclick={() => approveAndProcess(selectedRequest!, false)}
                  disabled={processing}
                >
                  <User size={14} />
                  HITL Fallback
                </Button>
                <Button
                  variant="primary"
                  size="sm"
                  onclick={() => approveAndProcess(selectedRequest!, true)}
                  disabled={processing}
                  loading={processing}
                >
                  <Bot size={14} />
                  {processing ? 'Processing...' : 'AI Test'}
                </Button>
              {/if}
            </div>
          {:else if selectedRequest.status === 'completed' || selectedRequest.status === 'error'}
            <div class="flex items-center gap-2">
              <Button
                variant="ghost"
                size="sm"
                onclick={() => copyToClipboard(JSON.stringify(selectedRequest, null, 2))}
              >
                <Copy size={14} />
                Copy All
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onclick={() => {
                  testMessage = formatMessageContent(selectedRequest!.messages.find(m => m.role === 'user')?.content || '');
                  testSystemPrompt = selectedRequest!.systemPrompt || '';
                  testMaxTokens = selectedRequest!.maxTokens || 500;
                  testTemperature = selectedRequest!.temperature || 0.7;
                }}
              >
                <RotateCcw size={14} />
                Retry
              </Button>
            </div>
          {/if}
        </div>
      </div>

      <div class="flex-1 overflow-hidden flex">
        <!-- Request Details Panel -->
        <div class="w-1/2 border-r border-gray-200 dark:border-gray-700 flex flex-col">
          <div class="p-4 border-b border-gray-200 dark:border-gray-700">
            <h4 class="font-medium text-gray-900 dark:text-white">Request Details</h4>
          </div>

          <div class="flex-1 overflow-y-auto p-4">
            <div class="space-y-6">

              <!-- Model Preferences -->
              {#if selectedRequest.modelPreferences}
                <div>
                  <h5 class="text-sm font-medium text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                    <Brain size={14} />
                    Model Preferences
                  </h5>
                  <div class="space-y-3">
                    {#if selectedRequest.modelPreferences.hints}
                      <div class="flex items-center justify-between">
                        <span class="text-xs text-gray-600 dark:text-gray-400">Preferred Model</span>
                        <span class="text-xs font-mono bg-blue-100 text-blue-700 px-2 py-1 rounded">
                          {selectedRequest.modelPreferences.hints[0]?.name || 'default'}
                        </span>
                      </div>
                    {/if}

                    {#each Object.entries(selectedRequest.modelPreferences) as [key, value]}
                      {#if key !== 'hints' && typeof value === 'number'}
                        <div class="flex items-center justify-between">
                          <span class="text-xs text-gray-600 dark:text-gray-400 capitalize">
                            {key.replace('Priority', '')}
                          </span>
                          <div class="flex items-center gap-2">
                            <div class="w-16 h-2 bg-gray-200 dark:bg-gray-600 rounded-full">
                              <div
                                class="h-2 bg-mcp-primary-500 rounded-full"
                                style="width: {value * 100}%"
                              ></div>
                            </div>
                            <span class="text-xs text-gray-500 w-8">
                              {Math.round(value * 100)}%
                            </span>
                          </div>
                        </div>
                      {/if}
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- System Prompt -->
              {#if selectedRequest.systemPrompt}
                <div>
                  <h5 class="text-sm font-medium text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                    <Settings size={14} />
                    System Prompt
                  </h5>
                  <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-3">
                    <pre class="text-xs font-mono text-gray-700 dark:text-gray-300 whitespace-pre-wrap">{selectedRequest.systemPrompt}</pre>
                  </div>
                </div>
              {/if}

              <!-- Parameters -->
              <div>
                <h5 class="text-sm font-medium text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                  <Settings size={14} />
                  Parameters
                </h5>
                <div class="grid grid-cols-2 gap-3">
                  {#if selectedRequest.maxTokens}
                    <div class="bg-gray-50 dark:bg-gray-700 rounded p-3">
                      <div class="text-xs text-gray-500 dark:text-gray-400">Max Tokens</div>
                      <div class="text-sm font-medium text-gray-900 dark:text-white">
                        {selectedRequest.maxTokens}
                      </div>
                    </div>
                  {/if}
                  {#if selectedRequest.temperature !== undefined}
                    <div class="bg-gray-50 dark:bg-gray-700 rounded p-3">
                      <div class="text-xs text-gray-500 dark:text-gray-400">Temperature</div>
                      <div class="text-sm font-medium text-gray-900 dark:text-white">
                        {selectedRequest.temperature}
                      </div>
                    </div>
                  {/if}
                </div>
              </div>

              <!-- Messages -->
              <div>
                <h5 class="text-sm font-medium text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                  <MessageSquare size={14} />
                  Messages ({selectedRequest.messages.length})
                </h5>
                <div class="space-y-3">
                  {#each selectedRequest.messages as message}
                    <div class="border border-gray-200 dark:border-gray-600 rounded-lg p-3">
                      <div class="flex items-center justify-between mb-2">
                        <div class="flex items-center gap-2">
                          {#if message.role === 'user'}
                            <User size={12} class="text-blue-600" />
                          {:else if message.role === 'assistant'}
                            <Bot size={12} class="text-green-600" />
                          {:else}
                            <Settings size={12} class="text-orange-600" />
                          {/if}
                          <span class="text-xs font-medium text-gray-700 dark:text-gray-300 capitalize">
                            {message.role}
                          </span>
                        </div>
                        <button
                          onclick={() => copyToClipboard(formatMessageContent(message.content))}
                          class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                        >
                          <Copy size={12} />
                        </button>
                      </div>
                      <div class="bg-gray-50 dark:bg-gray-700 rounded p-2">
                        <pre class="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap font-sans">{formatMessageContent(message.content)}</pre>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Response Panel -->
        <div class="w-1/2 flex flex-col">
          <div class="p-4 border-b border-gray-200 dark:border-gray-700">
            <div class="flex items-center justify-between">
              <h4 class="font-medium text-gray-900 dark:text-white">Response</h4>
              {#if selectedRequest.response}
                <button
                  onclick={() => copyToClipboard(formatMessageContent(selectedRequest!.response?.content))}
                  class="text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                  title="Copy response"
                >
                  <Copy size={14} />
                </button>
              {/if}
            </div>
          </div>

          <div class="flex-1 overflow-y-auto p-4">
            {#if processing && selectedRequest.status === 'approved'}
              <div class="flex items-center justify-center h-48">
                <div class="text-center">
                  <div class="relative">
                    <RefreshCw size={32} class="mx-auto text-mcp-primary-500 animate-spin mb-3" />
                    <div class="absolute inset-0 w-8 h-8 mx-auto border-2 border-mcp-primary-200 rounded-full animate-pulse"></div>
                  </div>
                  <p class="text-sm font-medium text-gray-900 dark:text-white mb-1">
                    🤖 Processing with AI...
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400">
                    Waiting for {llmConfig.provider} response
                  </p>
                  <div class="mt-3 flex items-center justify-center gap-1">
                    <div class="w-2 h-2 bg-mcp-primary-400 rounded-full animate-bounce" style="animation-delay: 0ms"></div>
                    <div class="w-2 h-2 bg-mcp-primary-400 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
                    <div class="w-2 h-2 bg-mcp-primary-400 rounded-full animate-bounce" style="animation-delay: 300ms"></div>
                  </div>
                </div>
              </div>
            {:else if selectedRequest.status === 'error' && selectedRequest.error}
              <!-- Error Diagnosis -->
              {@const currentRequest = selectedRequest}
              {@const diagnosis = diagnoseError(currentRequest.error, {
                estimatedTokens: currentRequest.cost ? Math.ceil(currentRequest.cost * 1000) : undefined,
                maxContextWindow: maxContextWindow,
                serverStatus: servers.find(s => s.id === currentRequest.serverId)?.status,
                hasApiKey: !!(activeProvider?.configured)
              })}

              <ErrorDiagnosis
                {diagnosis}
                onRetry={() => {
                  // Copy request params to test message for retry
                  testMessage = formatMessageContent(currentRequest.messages.find(m => m.role === 'user')?.content || '');
                  if (currentRequest.systemPrompt) testSystemPrompt = currentRequest.systemPrompt;
                  if (currentRequest.maxTokens) testMaxTokens = currentRequest.maxTokens;
                  if (currentRequest.temperature) testTemperature = currentRequest.temperature;

                  // Scroll to test message input
                  document.getElementById('test-message-input')?.scrollIntoView({ behavior: 'smooth' });
                  document.getElementById('test-message-input')?.focus();
                }}
              />

            {:else if selectedRequest.response}
              <div class="space-y-4">

                <!-- Response Metadata -->
                <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
                  <div class="grid grid-cols-2 gap-4">
                    {#if selectedRequest.response.model}
                      <div>
                        <div class="text-xs text-gray-500 dark:text-gray-400">Model Used</div>
                        <div class="text-sm font-medium text-gray-900 dark:text-white">
                          {selectedRequest.response.model}
                        </div>
                      </div>
                    {/if}
                    {#if selectedRequest.duration}
                      <div>
                        <div class="text-xs text-gray-500 dark:text-gray-400">Duration</div>
                        <div class="text-sm font-medium text-gray-900 dark:text-white">
                          {selectedRequest.duration}ms
                        </div>
                      </div>
                    {/if}
                    {#if selectedRequest.response.usage}
                      <div>
                        <div class="text-xs text-gray-500 dark:text-gray-400">Tokens</div>
                        <div class="text-sm font-medium text-gray-900 dark:text-white">
                          {selectedRequest.response.usage.inputTokens + selectedRequest.response.usage.outputTokens}
                          <span class="text-xs text-gray-500">
                            ({selectedRequest.response.usage.inputTokens} in, {selectedRequest.response.usage.outputTokens} out)
                          </span>
                        </div>
                      </div>
                    {/if}
                    {#if selectedRequest.cost}
                      <div>
                        <div class="text-xs text-gray-500 dark:text-gray-400">Est. Cost</div>
                        <div class="text-sm font-medium text-gray-900 dark:text-white">
                          ${selectedRequest.cost.toFixed(4)}
                        </div>
                      </div>
                    {/if}
                  </div>
                </div>

                <!-- Response Content -->
                <div class="border border-gray-200 dark:border-gray-600 rounded-lg p-4">
                  <div class="flex items-center gap-2 mb-3">
                    <Bot size={14} class="text-green-600" />
                    <span class="text-sm font-medium text-gray-900 dark:text-white">
                      Assistant Response
                    </span>
                    {#if selectedRequest.response.stopReason}
                      <span class="text-xs text-gray-500 dark:text-gray-400">
                        • {selectedRequest.response.stopReason}
                      </span>
                    {/if}
                  </div>
                  <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
                    <div class="prose prose-sm max-w-none">
                      <pre class="whitespace-pre-wrap font-sans text-sm text-gray-800 dark:text-gray-200">{formatMessageContent(selectedRequest.response.content)}</pre>
                    </div>
                  </div>
                </div>
              </div>
            {:else if selectedRequest.status === 'pending'}
              <div class="flex items-center justify-center h-48">
                <div class="text-center">
                  <Shield size={32} class="mx-auto text-orange-500 mb-3" />
                  <p class="text-sm font-medium text-gray-900 dark:text-white">
                    Awaiting Approval
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400">
                    {#if samplingMode === 'hitl'}
                      Choose "HITL Approve" to process manually
                    {:else if samplingMode === 'ai' && hasLLMConfig}
                      Choose "AI Test" for automatic processing or "HITL Fallback" for manual review
                    {:else}
                      Configure AI settings in Settings to enable automatic processing
                    {/if}
                  </p>
                </div>
              </div>
            {:else if selectedRequest.status === 'rejected'}
              <div class="flex items-center justify-center h-48">
                <div class="text-center">
                  <AlertCircle size={32} class="mx-auto text-red-500 mb-3" />
                  <p class="text-sm font-medium text-gray-900 dark:text-white">
                    Request Rejected
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400">
                    This sampling request was rejected
                  </p>
                </div>
              </div>
            {:else}
              <div class="flex items-center justify-center h-48">
                <div class="text-center">
                  <Eye size={32} class="mx-auto text-gray-400 mb-3" />
                  <p class="text-sm font-medium text-gray-900 dark:text-white">
                    No Response
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400">
                    No response available yet
                  </p>
                </div>
              </div>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <!-- No Request Selected -->
      <div class="flex items-center justify-center h-full">
        <div class="text-center max-w-md">
          <TestTube size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">
            MCP Sampling Workbench
          </h3>
          <p class="text-gray-600 dark:text-gray-400 mb-6">
            The ultimate tool for testing and debugging MCP sampling implementations.
            Create test requests and see how real LLMs respond to your server's sampling requests.
          </p>

          <div class="space-y-4 text-sm text-gray-600 dark:text-gray-400 text-left">
            <div class="flex items-start gap-3">
              <CheckCircle size={16} class="text-green-600 mt-0.5 flex-shrink-0" />
              <div>
                <strong class="text-gray-900 dark:text-white">HITL + AI Testing</strong>
                <br />Test with human approval or real AI responses
              </div>
            </div>
            <div class="flex items-start gap-3">
              <CheckCircle size={16} class="text-green-600 mt-0.5 flex-shrink-0" />
              <div>
                <strong class="text-gray-900 dark:text-white">Model Preference Testing</strong>
                <br />Validate your server's model preferences work correctly
              </div>
            </div>
            <div class="flex items-start gap-3">
              <CheckCircle size={16} class="text-green-600 mt-0.5 flex-shrink-0" />
              <div>
                <strong class="text-gray-900 dark:text-white">Conversation Context</strong>
                <br />See how conversation history affects responses
              </div>
            </div>
          </div>

          {#if !selectedServerId}
            <div class="mt-6">
              <Button
                variant="primary"
                onclick={() => uiStore.openModal('addServer')}
              >
                Connect MCP Server
              </Button>
            </div>
          {:else if !testMessage}
            <div class="mt-6">
              <p class="text-sm text-gray-600 dark:text-gray-400">
                Enter a test message in the left panel to get started
              </p>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .kbd {
    @apply inline-block px-1.5 py-0.5 text-xs font-mono bg-gray-200 text-gray-700 border border-gray-300 rounded shadow-sm;
  }

  .form-label {
    @apply block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1;
  }

  .form-input {
    @apply w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm
           bg-white dark:bg-gray-800 text-gray-900 dark:text-white
           focus:ring-2 focus:ring-mcp-primary-500 focus:border-mcp-primary-500
           placeholder:text-gray-400 dark:placeholder:text-gray-500;
  }

  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>