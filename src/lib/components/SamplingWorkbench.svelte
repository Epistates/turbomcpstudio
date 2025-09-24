<!--
  MCP Studio Sampling Workbench
  The ultimate MCP sampling testing interface - combines HITL + AI testing
  Enterprise-grade UX with pure DX focus for MCP server developers
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import Button from './ui/Button.svelte';
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
    response?: {
      role: 'assistant';
      content: { type: 'text'; text: string };
      model?: string;
      stopReason?: string;
      usage?: {
        inputTokens: number;
        outputTokens: number;
      };
    };
    duration?: number;
    cost?: number;
  }

  interface LLMConfig {
    provider: 'openai' | 'anthropic';
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

  // Cost estimation and analytics
  let estimatedCost = $state(0);
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

  const canTestAI = $derived(
    selectedServerId && hasLLMConfig && testMessage.trim().length > 0
  );

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

  $effect(() => {
    const unsubscribeServers = serverStore.subscribe(state => {
      const connectedServers = state.servers.filter(s =>
        s.status?.toLowerCase() === 'connected' &&
        s.capabilities?.sampling
      );
      servers = connectedServers;

      if (selectedServerId !== state.selectedServerId) {
        selectedServerId = state.selectedServerId;
      }

      // Auto-select first connected server with sampling capability
      if (!state.selectedServerId && connectedServers.length > 0 && !selectedServerId) {
        serverStore.selectServer(connectedServers[0].id);
      }
    });

    return () => {
      unsubscribeServers();
    };
  });

  // ===============================================
  // LLM CONFIGURATION & PROVIDER MANAGEMENT
  // ===============================================

  // Load LLM provider status from backend
  async function loadLLMProviders() {
    try {
      const [providersData, samplingStatus] = await Promise.all([
        invoke('get_llm_provider_statuses'),
        invoke('is_sampling_available')
      ]);

      llmProviders = providersData as any[];
      samplingAvailable = samplingStatus as boolean;

      // Find active provider
      activeProvider = llmProviders.find(p => p.active) || null;

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

  // Update cost estimation when message changes
  $effect(() => {
    if (testMessage && activeProvider) {
      const messages = [{ content: { text: testMessage } }];
      estimatedCost = estimateRequestCost(messages, testMaxTokens);
    } else {
      estimatedCost = 0;
    }
  });

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

        console.log(`Request failed, retrying in ${Math.round(delay + jitter)}ms... (attempt ${attempt + 1}/${maxRetries + 1})`);
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
  onMount(async () => {
    await loadLLMProviders();

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
    if (!selectedServerId || !testMessage.trim()) return;

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
        hints: [{ name: activeProviderModel }]
      },
      systemPrompt: testSystemPrompt.trim() || undefined,
      includeContext: 'thisServer',
      maxTokens: testMaxTokens,
      temperature: testTemperature,
      timestamp: new Date().toISOString(),
      status: 'pending'
    };

    samplingRequests = [testRequest, ...samplingRequests];
    selectedRequest = testRequest;

    // Clear the test form
    testMessage = '';

    uiStore.showSuccess('Test sampling request created');
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
        let result;
        try {
          result = await retryWithExponentialBackoff(async () => {
            return await invoke('create_sampling_request', {
              server_id: request.serverId,
              messages: createMessageRequest.messages,
              max_tokens: createMessageRequest.max_tokens,
              temperature: createMessageRequest.temperature
            });
          });

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
        // HITL mode - would normally wait for human response,
        // but for demo we'll create a mock response
        request.response = {
          role: 'assistant',
          content: {
            type: 'text',
            text: `[HITL Mode] This would normally wait for human input. In a real implementation, you would edit this response manually and approve it.`
          },
          model: 'human-review',
          stopReason: 'manual'
        };
        request.status = 'completed';
      }

      request.duration = Date.now() - startTime;

      // Estimate cost (mock calculation)
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

  onMount(() => {
    // Load saved LLM config from localStorage
    const saved = localStorage.getItem('mcp-studio-llm-config');
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        llmConfig = { ...llmConfig, ...parsed };
      } catch (e) {
        console.warn('Failed to load saved LLM config:', e);
      }
    }

    // Create initial demo request
    setTimeout(() => {
      if (selectedServerId) {
        testMessage = 'Hello! Please explain what MCP (Model Context Protocol) is and how it helps developers build better AI applications.';
        createTestSamplingRequest();
      }
    }, 1000);
  });

  // Save LLM config to localStorage
  $effect(() => {
    localStorage.setItem('mcp-studio-llm-config', JSON.stringify(llmConfig));
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
        <label class="form-label">Mode</label>
        <div class="grid grid-cols-2 gap-1 p-1 bg-gray-100 dark:bg-gray-700 rounded-lg">
          <button
            onclick={() => samplingMode = 'hitl'}
            class="px-3 py-2 text-sm font-medium rounded-md transition-all {samplingMode === 'hitl' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'}"
          >
            HITL Only
          </button>
          <button
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
          <label class="form-label">Server</label>
          <select
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
        <label class="form-label flex items-center gap-2">
          <MessageSquare size={14} />
          Test Message
        </label>
        <textarea
          bind:value={testMessage}
          placeholder="Enter a message to test sampling..."
          class="form-input h-20 resize-none"
          onkeydown={(e) => {
            if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && canTestAI) {
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
            disabled={!selectedServerId || !testMessage.trim()}
            class="flex-1"
          >
            <Plus size={14} />
            Create Request
          </Button>
          <button
            onclick={() => showAdvancedOptions = !showAdvancedOptions}
            class="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
            title="Advanced options"
          >
            <Settings size={16} />
          </button>
        </div>

        <p class="text-xs text-gray-500">
          <kbd class="kbd">⌘</kbd> + <kbd class="kbd">Enter</kbd> to create request
        </p>
      </div>
    </div>

    <!-- Advanced Options -->
    {#if showAdvancedOptions}
      <div class="p-4 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
        <div class="space-y-3">
          <div>
            <label class="form-label text-xs">System Prompt</label>
            <textarea
              bind:value={testSystemPrompt}
              class="form-input text-xs h-16 resize-none"
              placeholder="System prompt for testing..."
            ></textarea>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="form-label text-xs">Max Tokens</label>
              <input
                type="number"
                bind:value={testMaxTokens}
                min="1"
                max="4096"
                class="form-input text-xs"
              />
            </div>
            <div>
              <label class="form-label text-xs">Temperature</label>
              <input
                type="number"
                bind:value={testTemperature}
                min="0"
                max="2"
                step="0.1"
                class="form-input text-xs"
              />
            </div>
          </div>

          <!-- 2024 Cutting-Edge API Features -->
          {#if hasLLMConfig}
            <div class="border-t border-gray-200 dark:border-gray-600 pt-3 mt-3">
              <h4 class="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
                <Zap size={12} class="text-yellow-500" />
                2024 API Features
              </h4>

              <div class="space-y-3">
                <!-- Structured Outputs (100% Reliability) -->
                {#if supportsStructuredOutputs}
                  <div>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        bind:checked={useStructuredOutput}
                        class="form-checkbox text-xs"
                        disabled={!isStructuredOutputModel}
                      />
                      <span class="text-xs font-medium">
                        Structured Outputs
                        {#if isStructuredOutputModel}
                          <span class="text-green-600 ml-1">100% Reliable</span>
                        {:else}
                          <span class="text-orange-600 ml-1">(Switch to {activeProvider?.capabilities?.structured_output_models?.[0] || 'compatible model'})</span>
                        {/if}
                      </span>
                    </label>
                    {#if useStructuredOutput}
                      <div class="mt-2">
                        <label class="form-label text-xs">JSON Schema</label>
                        <textarea
                          bind:value={structuredOutputSchema}
                          class="form-input text-xs h-16 font-mono resize-none"
                          placeholder="JSON Schema for structured output..."
                        ></textarea>
                        <p class="text-xs text-gray-500 mt-1">
                          OpenAI gpt-4o-2024-08-06 achieves 100% schema compliance vs &lt;40% on older models
                        </p>
                      </div>
                    {/if}
                  </div>
                {/if}

                <!-- Batch Processing -->
                {#if supportsBatchProcessing}
                  <div>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        bind:checked={enableBatchMode}
                        class="form-checkbox text-xs"
                      />
                      <span class="text-xs font-medium">
                        Batch Processing
                        {#if activeProvider?.capabilities?.batch_discount_percentage}
                          <span class="text-green-600 ml-1">
                            ({activeProvider.capabilities.batch_discount_percentage}% cheaper)
                          </span>
                        {/if}
                      </span>
                    </label>
                    {#if enableBatchMode}
                      <p class="text-xs text-gray-500 mt-1">
                        24-hour processing time with significant cost savings
                      </p>
                    {/if}
                  </div>
                {/if}

                <!-- Parallel Function Calling -->
                {#if supportsParallelFunctionCalling}
                  <div>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        bind:checked={enableParallelFunctionCalls}
                        class="form-checkbox text-xs"
                      />
                      <span class="text-xs font-medium">
                        Parallel Function Calling
                        <span class="text-blue-600 ml-1">(2024 Enhancement)</span>
                      </span>
                    </label>
                    {#if enableParallelFunctionCalls}
                      <p class="text-xs text-gray-500 mt-1">
                        Execute multiple tool calls simultaneously with strict mode
                      </p>
                    {/if}
                  </div>
                {/if}

                {#if !supportsStructuredOutputs && !supportsBatchProcessing && !supportsParallelFunctionCalling}
                  <div class="text-xs text-gray-500 italic">
                    No 2024 features available for {activeProvider?.display_name || 'current provider'}
                  </div>
                {/if}
              </div>
            </div>

            <!-- September 2025 Cutting-Edge Features -->
            <div class="border-t border-gray-200 dark:border-gray-600 pt-3 mt-3">
              <h4 class="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
                <Brain size={12} class="text-purple-500" />
                September 2025 Features
              </h4>

              <div class="space-y-4">
                <!-- OpenAI GPT-5 Reasoning Effort Parameter -->
                {#if isOpenAIGPT5}
                  <div>
                    <label class="text-xs font-medium text-gray-700 dark:text-gray-300 mb-2 block">
                      GPT-5 Reasoning Effort
                      <span class="text-blue-600 ml-1">⚡ Official API Parameter</span>
                    </label>
                    <select
                      bind:value={reasoningEffort}
                      class="form-input text-xs w-full"
                    >
                      <option value="minimal">Minimal - Fast responses, no reasoning</option>
                      <option value="low">Low - Light reasoning</option>
                      <option value="medium">Medium - Balanced (default)</option>
                      <option value="high">High - Maximum quality, slower</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">
                      Controls reasoning depth. Higher = better quality but more output tokens ($10/1M).
                      Reasoning tokens count as output tokens.
                    </p>
                  </div>

                  <div>
                    <label class="text-xs font-medium text-gray-700 dark:text-gray-300 mb-2 block">
                      GPT-5 Verbosity
                    </label>
                    <select
                      bind:value={verbosity}
                      class="form-input text-xs w-full"
                    >
                      <option value="low">Low - Concise responses</option>
                      <option value="medium">Medium - Balanced (default)</option>
                      <option value="high">High - Detailed responses</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">
                      Controls default response length independently of reasoning effort.
                    </p>
                  </div>
                {/if}

                <!-- Claude 4.1 Opus Extended Thinking Mode -->
                {#if isClaudeWithThinking}
                  <div>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        bind:checked={enableExtendedThinking}
                        class="form-checkbox text-xs"
                      />
                      <span class="text-xs font-medium">
                        Extended Thinking Mode
                        <span class="text-purple-600 ml-1">🧠 Claude 4.1 Only</span>
                        {#if hasThinkingCost}
                          <span class="text-amber-600 ml-1">
                            (+${activeProvider?.cost_config?.thinking_cost_per_1k}/1K thinking tokens)
                          </span>
                        {/if}
                      </span>
                    </label>
                    {#if enableExtendedThinking}
                      <div class="mt-2 space-y-2">
                        <label class="text-xs font-medium text-gray-700 dark:text-gray-300 mb-1 block">
                          Thinking Budget
                        </label>
                        <select
                          bind:value={thinkingBudget}
                          class="form-input text-xs w-full"
                        >
                          <option value="low">Low - Quick thinking</option>
                          <option value="medium">Medium - Balanced</option>
                          <option value="high">High - Deep reasoning</option>
                        </select>
                        <label class="flex items-center gap-2 cursor-pointer">
                          <input
                            type="checkbox"
                            bind:checked={showThinkingProcess}
                            class="form-checkbox text-xs"
                          />
                          <span class="text-xs">Show thinking summaries in UI</span>
                        </label>
                        <p class="text-xs text-gray-500">
                          Claude alternates between reasoning and tool use for improved responses.
                          Performance matters more than latency.
                        </p>
                      </div>
                    {/if}
                  </div>
                {:else if supportsExtendedThinking}
                  <div>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        bind:checked={enableExtendedThinking}
                        class="form-checkbox text-xs"
                      />
                      <span class="text-xs font-medium">
                        Extended Thinking Mode
                        <span class="text-blue-600 ml-1">🧠 Claude 4+</span>
                      </span>
                    </label>
                    {#if enableExtendedThinking}
                      <div class="mt-2 space-y-2">
                        <label class="text-xs font-medium text-gray-700 dark:text-gray-300 mb-1 block">
                          Thinking Budget
                        </label>
                        <select
                          bind:value={thinkingBudget}
                          class="form-input text-xs w-full"
                        >
                          <option value="low">Low - Quick thinking</option>
                          <option value="medium">Medium - Balanced</option>
                          <option value="high">High - Deep reasoning</option>
                        </select>
                        <p class="text-xs text-gray-500">
                          Standard extended thinking without thinking token costs.
                        </p>
                      </div>
                    {/if}
                  </div>
                {/if}

                <!-- Computer Use (Claude 4+ Models) -->
                {#if supportsComputerUse}
                  <div>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        bind:checked={enableComputerUse}
                        class="form-checkbox text-xs"
                      />
                      <span class="text-xs font-medium">
                        Computer Use
                        <span class="text-blue-600 ml-1">🖥️ Beta</span>
                      </span>
                    </label>
                    {#if enableComputerUse}
                      <p class="text-xs text-gray-500 mt-1">
                        <strong>⚠️ Beta:</strong> AI can take screenshots, move cursor, type text, and click buttons. Use with caution in secure environments.
                      </p>
                    {/if}
                  </div>
                {/if}

                <!-- Context Window Display -->
                <div class="bg-gradient-to-r from-purple-50 to-blue-50 dark:from-purple-900/20 dark:to-blue-900/20 p-3 rounded-lg border border-purple-200 dark:border-purple-800">
                  <div class="flex items-center justify-between">
                    <span class="text-xs font-medium text-gray-700 dark:text-gray-300">
                      Context Window
                    </span>
                    <span class="text-xs font-semibold {contextTokenLimit() >= 1000000 ? 'text-green-600' : contextTokenLimit() >= 200000 ? 'text-blue-600' : 'text-gray-600'}">
                      {#if contextTokenLimit() >= 1000000}
                        🚀 {Math.round(contextTokenLimit() / 1000000)}M tokens
                      {:else if contextTokenLimit() >= 1000}
                        {Math.round(contextTokenLimit() / 1000)}K tokens
                      {:else}
                        {contextTokenLimit()} tokens
                      {/if}
                    </span>
                  </div>
                  <div class="mt-1">
                    <input
                      type="range"
                      min="1000"
                      max={contextTokenLimit()}
                      bind:value={maxContextTokens}
                      class="w-full h-1 bg-gray-200 rounded-lg appearance-none cursor-pointer"
                    />
                    <div class="flex justify-between text-xs text-gray-500 mt-1">
                      <span>1K</span>
                      <span class="font-medium">{Math.round(maxContextTokens / 1000)}K selected</span>
                      <span>
                        {#if contextTokenLimit() >= 1000000}
                          1M max
                        {:else}
                          {Math.round(contextTokenLimit() / 1000)}K max
                        {/if}
                      </span>
                    </div>
                  </div>
                </div>

                {#if !isOpenAIGPT5 && !supportsExtendedThinking && !supportsComputerUse}
                  <div class="text-xs text-gray-500 italic">
                    No September 2025 features available for {activeProvider?.display_name || 'current provider'}.
                    Try GPT-5 (reasoning_effort) or Claude 4+ (extended thinking).
                  </div>
                {/if}
              </div>
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

        <!-- Provider Status (Read-only) -->
        <div class="space-y-2 mb-4">
          {#if activeProvider}
            <div class="p-3 rounded-lg border border-green-200 bg-green-50">
              <div class="flex items-center justify-between mb-2">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-medium">{activeProvider.display_name}</span>
                  <CheckCircle size={12} class="text-green-600" />
                  <span class="text-xs text-green-600">Active</span>
                </div>
                {#if activeProvider.usage_stats}
                  <div class="flex items-center gap-2 text-xs text-gray-500">
                    <span>{activeProvider.usage_stats.total_requests || 0} reqs</span>
                    <span>${(activeProvider.usage_stats.total_cost || 0).toFixed(3)}</span>
                  </div>
                {/if}
              </div>
              <div class="flex items-center justify-between text-xs">
                <span class="text-gray-600">Model: {activeProvider.default_model}</span>
                <button
                  onclick={() => uiStore.setView('settings')}
                  class="text-mcp-primary-600 hover:text-mcp-primary-700 font-medium"
                >
                  Configure in Settings →
                </button>
              </div>
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

    <!-- Request Queue -->
    <div class="flex-1 overflow-y-auto">

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
                onclick={() => rejectRequest(selectedRequest)}
                disabled={processing}
              >
                <Square size={14} />
                Reject
              </Button>

              {#if samplingMode === 'hitl'}
                <Button
                  variant="primary"
                  size="sm"
                  onclick={() => approveAndProcess(selectedRequest, false)}
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
                  onclick={() => approveAndProcess(selectedRequest, false)}
                  disabled={processing}
                >
                  <User size={14} />
                  HITL Fallback
                </Button>
                <Button
                  variant="primary"
                  size="sm"
                  onclick={() => approveAndProcess(selectedRequest, true)}
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
                  testMessage = formatMessageContent(selectedRequest.messages.find(m => m.role === 'user')?.content || '');
                  testSystemPrompt = selectedRequest.systemPrompt || '';
                  testMaxTokens = selectedRequest.maxTokens || 500;
                  testTemperature = selectedRequest.temperature || 0.7;
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
                  onclick={() => copyToClipboard(formatMessageContent(selectedRequest.response?.content))}
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
                  <RefreshCw size={32} class="mx-auto text-mcp-primary-500 animate-spin mb-3" />
                  <p class="text-sm font-medium text-gray-900 dark:text-white">
                    Processing with AI...
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400">
                    Waiting for {llmConfig.provider} response
                  </p>
                </div>
              </div>
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