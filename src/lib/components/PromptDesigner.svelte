<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo, type ToolExecution } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import {
    FileText,
    Play,
    Search,
    RefreshCw,
    AlertCircle,
    CheckCircle,
    Clock,
    Edit,
    Copy,
    Download,
    Upload,
    Settings,
    ChevronDown,
    ChevronRight,
    Variable,
    Eye,
    Save,
    Star,
    Bookmark,
    History
  } from 'lucide-svelte';

  interface PromptTemplate {
    name: string;
    title?: string;
    description?: string;
    arguments?: Array<{
      name: string;
      description?: string;
      required: boolean;
      type?: string;
      default?: any;
    }>;
  }

  interface PromptMessage {
    role: 'user' | 'assistant' | 'system';
    content: any; // Can be string or content array
  }

  interface PromptResult {
    description?: string;
    messages: PromptMessage[];
  }


  let servers: ServerInfo[] = $state([]);
  let selectedServerId: string | undefined = $state(undefined);
  let prompts: PromptTemplate[] = $state([]);
  let loading = $state(false);
  let searchQuery = $state('');
  let selectedPrompt: PromptTemplate | null = $state(null);
  let promptArguments = $state<Record<string, any>>({});
  let promptResult: PromptResult | null = $state(null);
  let executingPrompt = $state(false);
  let expandedPrompt: string | null = $state(null);
  let savedPrompts: Array<{id: string; name: string; template: PromptTemplate; arguments: Record<string, any>}> = $state([]);
  // Get execution history from centralized store (filtered for prompts)
  let serverStoreState = $state(null);
  const allExecutionHistory = $derived(
    serverStoreState?.toolExecutions?.filter(e => e.tool.startsWith('prompt:')) || []
  );

  // Filter execution history by selected prompt
  const executionHistory = $derived(
    selectedPrompt
      ? allExecutionHistory.filter(e => e.tool === `prompt:${selectedPrompt.name}`)
      : allExecutionHistory
  );
  let isHistoricalResult = $state(false);

  const selectedServer = $derived(
    servers.find(s => s.id === selectedServerId)
  );

  // Subscribe to stores
  $effect(() => {
    const unsubscribeServers = serverStore.subscribe(state => {
      const connectedServers = state.servers.filter(s => s.status?.toLowerCase() === 'connected');
      servers = connectedServers;
      serverStoreState = state; // Update reactive state for execution history

      if (selectedServerId !== state.selectedServerId) {
        selectedServerId = state.selectedServerId;
        if (selectedServerId) {
          loadPrompts();
        }
      }

      // Auto-select first connected server if none selected
      if (!state.selectedServerId && connectedServers.length > 0 && !selectedServerId) {
        serverStore.selectServer(connectedServers[0].id);
      }
    });

    return () => {
      unsubscribeServers();
    };
  });

  const filteredPrompts = $derived.by(() => {
    console.log('🔍 PROMPT FILTER DEBUG: Starting with prompts:', prompts.length);
    if (!searchQuery.trim()) {
      console.log('🔍 PROMPT FILTER DEBUG: No search query, returning all prompts:', prompts.length);
      return prompts;
    }

    const query = searchQuery.toLowerCase();
    const filtered = prompts.filter(prompt =>
      prompt.name.toLowerCase().includes(query) ||
      prompt.title?.toLowerCase().includes(query) ||
      prompt.description?.toLowerCase().includes(query)
    );
    console.log('🔍 PROMPT FILTER DEBUG: After search filter:', filtered.length);
    return filtered;
  });

  async function loadPrompts() {
    if (!selectedServerId) return;

    loading = true;
    try {
      // Check if server supports prompts capability
      const serverInfo = await invoke('get_server_info', { serverId: selectedServerId });
      console.log('🔍 PROMPT DEBUG: Server info received:', serverInfo);
      console.log('🔍 PROMPT DEBUG: Capabilities:', serverInfo.capabilities);
      console.log('🔍 PROMPT DEBUG: Prompts capability:', serverInfo.capabilities?.prompts);
      console.log('🔍 PROMPT DEBUG: typeof capabilities:', typeof serverInfo.capabilities);
      console.log('🔍 PROMPT DEBUG: typeof prompts:', typeof serverInfo.capabilities?.prompts);
      console.log('🔍 PROMPT DEBUG: prompts truthy?:', !!serverInfo.capabilities?.prompts);
      console.log('🔍 PROMPT DEBUG: JSON.stringify capabilities:', JSON.stringify(serverInfo.capabilities, null, 2));

      if (!serverInfo.capabilities?.prompts) {
        prompts = [];
        console.log('❌ PROMPT DEBUG: No prompts capability found, showing info message');
        uiStore.showInfo('This server does not support prompts operations. Try the Tools tab instead.');
        return;
      }

      console.log('✅ PROMPT DEBUG: Capability check passed, calling list_prompts...');
      const promptList = await invoke('list_prompts', { serverId: selectedServerId });
      console.log('✅ PROMPT DEBUG: list_prompts call succeeded, got:', promptList);
      prompts = promptList.map((prompt: any) => ({
        name: prompt.name,
        title: prompt.title,
        description: prompt.description,
        arguments: prompt.arguments ? prompt.arguments.map((arg: any) => ({
          name: arg.name,
          description: arg.description,
          required: arg.required || false,
          type: arg.type || 'string',
          default: arg.default
        })) : []
      }));

      console.log('✅ PROMPT DEBUG: Processed prompts array:', prompts);
      console.log('✅ PROMPT DEBUG: Prompts length:', prompts.length);
      uiStore.showSuccess(`Loaded ${prompts.length} prompts`);
    } catch (error) {
      console.error('Failed to load prompts:', error);
      const errorStr = String(error);

      if (errorStr.includes('Method not found') || errorStr.includes('-32601')) {
        uiStore.showError('This server does not implement prompts operations. Check server capabilities or try a different MCP server.');
      } else {
        uiStore.showError(`Failed to load prompts: ${error}`);
      }
      prompts = [];
    } finally {
      loading = false;
    }
  }

  function selectPrompt(prompt: PromptTemplate) {
    selectedPrompt = prompt;
    promptResult = null;
    isHistoricalResult = false;

    // Initialize arguments with defaults
    promptArguments = {};
    if (prompt.arguments) {
      prompt.arguments.forEach(arg => {
        promptArguments[arg.name] = arg.default || (arg.type === 'string' ? '' : null);
      });
    }
  }

  async function executePrompt() {
    if (!selectedPrompt || !selectedServerId) return;

    const startTime = Date.now();
    const executionId = crypto.randomUUID();

    executingPrompt = true;
    isHistoricalResult = false;
    let executionStatus: 'success' | 'error' = 'success';
    let executionError: string | undefined;

    try {
      console.log('🔍 PROMPT DEBUG: About to call get_prompt with:', {
        serverId: selectedServerId,
        promptName: selectedPrompt.name,
        parameters: promptArguments,
        selectedPrompt: selectedPrompt
      });

      const result = await invoke('get_prompt', {
        serverId: selectedServerId,
        promptName: selectedPrompt.name,
        parameters: promptArguments
      });

      // Handle the MCP prompt response format
      if (result.description || result.messages) {
        promptResult = {
          description: result.description,
          messages: result.messages || []
        };
      } else {
        // Fallback format if server returns unexpected structure
        promptResult = {
          description: `Generated prompt for ${selectedPrompt.title || selectedPrompt.name}`,
          messages: [
            {
              role: 'user',
              content: typeof result === 'string' ? result : JSON.stringify(result, null, 2)
            }
          ]
        };
      }

      uiStore.showSuccess(`Prompt "${selectedPrompt.title || selectedPrompt.name}" executed successfully`);
    } catch (error) {
      console.error('Prompt execution failed:', error);
      executionStatus = 'error';
      executionError = String(error);
      uiStore.showError(`Failed to execute prompt: ${error}`);

      // Show error in result panel for debugging
      promptResult = {
        description: 'Prompt execution failed',
        messages: [
          {
            role: 'system',
            content: `Error: ${error}`
          }
        ]
      };
    } finally {
      executingPrompt = false;

      // Add to execution history using centralized system
      if (promptResult) {
        serverStore.addPromptExecution(
          selectedServerId!,
          selectedPrompt.name,
          { ...promptArguments },
          promptResult,
          Date.now() - startTime,
          executionStatus,
          executionError
        );
      }
    }
  }

  function rerunFromHistory(historyItem: ToolExecution) {
    // Extract prompt name from tool field (format: "prompt:promptName")
    const promptName = historyItem.tool.replace('prompt:', '');
    const prompt = prompts.find(p => p.name === promptName);
    if (prompt) {
      selectedPrompt = prompt;
      promptArguments = { ...historyItem.parameters };
      promptResult = historyItem.result;
      isHistoricalResult = true;
    }
  }

  function formatContent(content: any): string {
    if (typeof content === 'string') {
      // Handle escape sequences and make newlines visible
      return content
        .replace(/\\n/g, '\n')  // Convert \n to actual newlines
        .replace(/\\t/g, '\t')  // Convert \t to actual tabs
        .replace(/\\r/g, '\r')  // Convert \r to actual carriage returns
        .replace(/\\\\/g, '\\'); // Convert \\\\ to single backslash
    } else if (typeof content === 'object' && content !== null) {
      // Pretty print JSON with proper formatting
      return JSON.stringify(content, null, 2);
    } else {
      return String(content);
    }
  }

  function generatePromptContent(prompt: PromptTemplate, args: Record<string, any>): string {
    // Generate realistic prompt content based on template and arguments
    switch (prompt.name) {
      case 'code_review':
        return `Please review the following ${args.language || 'code'} with focus on ${args.focus || 'all areas'}:\n\n${args.code || '[Code would be inserted here]'}`;
      case 'documentation_generator':
        return `Generate ${args.format || 'markdown'} documentation at ${args.level || 'detailed'} level for:\n\n${args.content || '[Content would be inserted here]'}`;
      case 'task_planner':
        return `Break down this goal into actionable steps:\n\nGoal: ${args.goal || '[Goal would be inserted here]'}\nTimeframe: ${args.timeframe || 'flexible'}\nResources: ${args.resources || 'standard'}`;
      default:
        return `Execute prompt "${prompt.name}" with arguments: ${JSON.stringify(args, null, 2)}`;
    }
  }

  function togglePromptExpansion(promptName: string) {
    expandedPrompt = expandedPrompt === promptName ? null : promptName;
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    uiStore.showSuccess('Copied to clipboard');
  }

  function savePromptConfiguration() {
    if (!selectedPrompt) return;

    const savedConfig = {
      id: crypto.randomUUID(),
      name: `${selectedPrompt.title || selectedPrompt.name} - ${new Date().toLocaleDateString()}`,
      template: selectedPrompt,
      arguments: { ...promptArguments }
    };

    savedPrompts = [savedConfig, ...savedPrompts];
    uiStore.showSuccess('Prompt configuration saved');
  }

  function loadSavedPrompt(saved: any) {
    selectPrompt(saved.template);
    promptArguments = { ...saved.arguments };
  }

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
  }

  function getArgumentType(arg: any): string {
    switch (arg.type) {
      case 'string':
        return arg.description?.includes('large') || arg.description?.includes('text') ? 'textarea' : 'text';
      case 'number':
      case 'integer':
        return 'number';
      case 'boolean':
        return 'checkbox';
      default:
        return 'text';
    }
  }

  function formatMessageContent(content: any): string {
    return formatContent(content);
  }

  onMount(() => {
    if (selectedServerId) {
      loadPrompts();
    }
  });
</script>

<div class="h-full flex bg-gray-50">
  <!-- Left Panel: Prompt List -->
  <div class="w-1/3 bg-white border-r border-gray-200 flex flex-col">
    <!-- Header -->
    <div class="p-4 border-b border-gray-200">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-lg font-semibold text-gray-900">Prompts</h2>
        <button
          onclick={loadPrompts}
          disabled={loading || !selectedServerId}
          class="btn-secondary {loading ? 'opacity-50' : ''}"
        >
          <RefreshCw size={16} class="{loading ? 'animate-spin' : ''}" />
        </button>
      </div>

      <!-- Server Selection -->
      {#if servers.length > 1}
        <div class="mb-3">
          <label class="block text-xs font-medium text-gray-700 mb-1">Server</label>
          <select
            bind:value={selectedServerId}
            onchange={(e) => selectServer(e.currentTarget.value)}
            class="form-select text-sm"
          >
            {#each servers as server}
              <option value={server.id}>{server.config.name}</option>
            {/each}
          </select>
        </div>
      {/if}

      <!-- Search -->
      <div class="relative">
        <Search size={16} class="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" />
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search prompts..."
          class="form-input pl-10 text-sm"
        />
      </div>
    </div>

    <!-- Prompt List -->
    <div class="flex-1 overflow-y-auto">
      {#if loading}
        <div class="flex items-center justify-center p-8">
          <RefreshCw size={24} class="animate-spin text-gray-400 mr-3" />
          <span class="text-gray-600">Loading prompts...</span>
        </div>
      {:else if !selectedServerId}
        <div class="text-center p-8">
          <FileText size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-medium text-gray-900 mb-2">No Server Selected</h3>
          <p class="text-gray-600">Select a connected server to view prompts</p>
        </div>
      {:else if filteredPrompts.length === 0}
        <div class="text-center p-8">
          <FileText size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-medium text-gray-900 mb-2">No Prompts Found</h3>
          <p class="text-gray-600">
            {searchQuery ? 'No prompts match your search' : 'This server has no available prompts'}
          </p>
        </div>
      {:else}
        <div class="p-4 space-y-3">
          {#each filteredPrompts as prompt}
            <div class="border border-gray-200 rounded-lg overflow-hidden">
              <div class="w-full p-3 bg-gray-50 hover:bg-gray-100 transition-colors
                         {selectedPrompt?.name === prompt.name ? 'ring-2 ring-mcp-primary-500 bg-mcp-primary-50' : ''}">
                <div class="flex items-start justify-between">
                  <button
                    onclick={() => selectPrompt(prompt)}
                    class="flex-1 min-w-0 text-left"
                  >
                    <h4 class="text-sm font-medium text-gray-900 truncate">
                      {prompt.title || prompt.name}
                    </h4>
                    {#if prompt.description}
                      <p class="text-xs text-gray-600 mt-1 line-clamp-2">
                        {prompt.description}
                      </p>
                    {/if}
                    {#if prompt.arguments && prompt.arguments.length > 0}
                      <div class="flex items-center space-x-2 mt-2">
                        <Variable size={12} class="text-gray-400" />
                        <span class="text-xs text-gray-500">
                          {prompt.arguments.length} argument{prompt.arguments.length === 1 ? '' : 's'}
                        </span>
                      </div>
                    {/if}
                  </button>

                  <button
                    onclick={() => togglePromptExpansion(prompt.name)}
                    class="ml-2 p-1 text-gray-400 hover:text-gray-600 flex-shrink-0"
                  >
                    {#if expandedPrompt === prompt.name}
                      <ChevronDown size={14} />
                    {:else}
                      <ChevronRight size={14} />
                    {/if}
                  </button>
                </div>
              </div>

              {#if expandedPrompt === prompt.name && prompt.arguments}
                <div class="border-t border-gray-200 bg-gray-50 p-3">
                  <h5 class="text-xs font-medium text-gray-700 mb-2">Arguments:</h5>
                  <div class="space-y-2">
                    {#each prompt.arguments as arg}
                      <div class="text-xs">
                        <div class="flex items-center space-x-2">
                          <span class="font-medium text-gray-900">{arg.name}</span>
                          {#if arg.required}
                            <span class="text-red-500">*</span>
                          {/if}
                          {#if arg.type}
                            <span class="text-gray-500">({arg.type})</span>
                          {/if}
                        </div>
                        {#if arg.description}
                          <p class="text-gray-600 mt-1">{arg.description}</p>
                        {/if}
                        {#if arg.default !== undefined}
                          <p class="text-gray-500 mt-1">Default: {arg.default}</p>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Saved Prompts Section -->
    {#if savedPrompts.length > 0}
      <div class="border-t border-gray-200 bg-gray-50 p-4">
        <h3 class="text-sm font-medium text-gray-900 mb-3 flex items-center">
          <Bookmark size={14} class="mr-2" />
          Saved Configurations
        </h3>
        <div class="space-y-2 max-h-32 overflow-y-auto">
          {#each savedPrompts.slice(0, 5) as saved}
            <button
              onclick={() => loadSavedPrompt(saved)}
              class="w-full p-2 text-left bg-white hover:bg-gray-100 rounded border border-gray-200 transition-colors"
            >
              <p class="text-xs font-medium text-gray-900 truncate">{saved.name}</p>
              <p class="text-xs text-gray-500">{saved.template.title || saved.template.name}</p>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <!-- Right Panel: Prompt Designer -->
  <div class="flex-1 flex flex-col bg-white">
    {#if selectedPrompt}
      <!-- Header -->
      <div class="p-4 border-b border-gray-200">
        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-lg font-semibold text-gray-900">
              {selectedPrompt.title || selectedPrompt.name}
            </h3>
            {#if selectedPrompt.description}
              <p class="text-sm text-gray-600 mt-1">{selectedPrompt.description}</p>
            {/if}
          </div>

          <div class="flex items-center space-x-2">
            <button
              onclick={savePromptConfiguration}
              class="btn-secondary text-sm"
              title="Save Configuration"
            >
              <Save size={14} />
            </button>
            <button
              onclick={executePrompt}
              disabled={executingPrompt}
              class="btn-primary text-sm"
            >
              <Play size={14} class="{executingPrompt ? 'animate-pulse' : ''} mr-1" />
              {executingPrompt ? 'Executing...' : 'Execute'}
            </button>
          </div>
        </div>
      </div>

      <div class="flex-1 overflow-hidden">
        <div class="h-full flex">
          <!-- Arguments Panel -->
          <div class="w-1/2 border-r border-gray-200 p-4 overflow-y-auto">
            <h4 class="text-sm font-medium text-gray-900 mb-4">Arguments</h4>

            {#if selectedPrompt.arguments && selectedPrompt.arguments.length > 0}
              <div class="space-y-4">
                {#each selectedPrompt.arguments as arg}
                  <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">
                      {arg.name}
                      {#if arg.required}
                        <span class="text-red-500">*</span>
                      {/if}
                    </label>

                    {#if arg.description}
                      <p class="text-xs text-gray-600 mb-2">{arg.description}</p>
                    {/if}

                    {#if getArgumentType(arg) === 'textarea'}
                      <textarea
                        bind:value={promptArguments[arg.name]}
                        placeholder={arg.default ? `Default: ${arg.default}` : ''}
                        class="form-input h-24 resize-none"
                        required={arg.required}
                      ></textarea>
                    {:else if getArgumentType(arg) === 'checkbox'}
                      <label class="flex items-center">
                        <input
                          type="checkbox"
                          bind:checked={promptArguments[arg.name]}
                          class="form-checkbox"
                        />
                        <span class="ml-2 text-sm text-gray-700">Enable</span>
                      </label>
                    {:else}
                      <input
                        type={getArgumentType(arg)}
                        bind:value={promptArguments[arg.name]}
                        placeholder={arg.default ? `Default: ${arg.default}` : ''}
                        class="form-input"
                        required={arg.required}
                      />
                    {/if}
                  </div>
                {/each}
              </div>
            {:else}
              <div class="text-center py-8">
                <Variable size={32} class="mx-auto text-gray-400 mb-2" />
                <p class="text-gray-600">This prompt has no arguments</p>
              </div>
            {/if}
          </div>

          <!-- Result Panel -->
          <div class="w-1/2 p-4 overflow-y-auto">
            <div class="flex items-center justify-between mb-4">
              <div class="flex items-center space-x-2">
                <h4 class="text-sm font-medium text-gray-900">Prompt Result</h4>
                {#if isHistoricalResult}
                  <span class="inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-blue-100 text-blue-800">
                    <History size={12} class="mr-1" />
                    Historical Result
                  </span>
                {/if}
              </div>
              {#if promptResult}
                <button
                  onclick={() => copyToClipboard(JSON.stringify(promptResult, null, 2))}
                  class="btn-secondary text-sm"
                >
                  <Copy size={14} class="mr-1" />
                  Copy
                </button>
              {/if}
            </div>

            {#if executingPrompt}
              <div class="flex items-center justify-center py-12">
                <RefreshCw size={24} class="animate-spin text-gray-400 mr-3" />
                <span class="text-gray-600">Executing prompt...</span>
              </div>
            {:else if promptResult}
              <div class="space-y-4">
                {#if promptResult.description}
                  <div class="bg-blue-50 border border-blue-200 rounded-lg p-3">
                    <p class="text-sm text-blue-800">{promptResult.description}</p>
                  </div>
                {/if}

                <div class="space-y-3">
                  {#each promptResult.messages as message, index}
                    <div class="border border-gray-200 rounded-lg p-3">
                      <div class="flex items-center justify-between mb-2">
                        <span class="text-xs font-medium text-gray-700 uppercase tracking-wide">
                          {message.role}
                        </span>
                        <button
                          onclick={() => copyToClipboard(formatMessageContent(message.content))}
                          class="text-gray-400 hover:text-gray-600"
                        >
                          <Copy size={12} />
                        </button>
                      </div>
                      <div class="bg-gray-50 rounded p-3 font-mono text-sm">
                        <pre class="whitespace-pre-wrap">{formatMessageContent(message.content)}</pre>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {:else}
              <div class="flex items-center justify-center py-12 text-gray-500">
                <Eye size={24} class="mr-2" />
                Click "Execute" to generate prompt
              </div>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <!-- No Prompt Selected -->
      <div class="flex items-center justify-center h-full">
        <div class="text-center">
          <FileText size={48} class="mx-auto text-gray-400 mb-4" />
          <h3 class="text-lg font-semibold text-gray-900 mb-2">Select a Prompt</h3>
          <p class="text-gray-600">Choose a prompt from the list to configure and execute it</p>
        </div>
      </div>
    {/if}

    <!-- Execution History -->
    {#if executionHistory.length > 0}
      <div class="border-t border-gray-200">
        <div class="p-4">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-sm font-medium text-gray-900 flex items-center">
              <History size={16} class="mr-2" />
              Execution History ({executionHistory.length})
            </h3>
            <button
              onclick={() => serverStore.clearExecutionHistory()}
              class="text-xs text-red-600 hover:text-red-800"
            >
              Clear History
            </button>
          </div>

          <div class="space-y-2 max-h-64 overflow-y-auto">
            {#each executionHistory as execution}
              <div
                onclick={() => rerunFromHistory(execution)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); rerunFromHistory(execution); } }}
                role="button"
                tabindex="0"
                class="w-full text-left bg-gray-50 hover:bg-gray-100 border border-gray-200 rounded-lg p-3 transition-colors cursor-pointer"
              >
                <div class="flex items-center justify-between mb-2">
                  <div class="flex items-center space-x-2">
                    <span class="text-xs font-mono bg-blue-100 text-blue-800 px-2 py-1 rounded">
                      {execution.tool.replace('prompt:', '')}
                    </span>
                    <span class="text-xs text-gray-500">
                      {execution.serverName}
                    </span>
                    {#if execution.status === 'success'}
                      <CheckCircle size={12} class="text-green-500" />
                    {:else}
                      <AlertCircle size={12} class="text-red-500" />
                    {/if}
                  </div>
                  <div class="flex items-center space-x-2">
                    <span class="text-xs text-gray-500">
                      {new Date(execution.timestamp).toLocaleTimeString()}
                    </span>
                    {#if execution.duration}
                      <span class="text-xs text-gray-500">
                        ({execution.duration}ms)
                      </span>
                    {/if}
                  </div>
                </div>

                {#if Object.keys(execution.parameters).length > 0}
                  <div class="text-xs text-gray-600 mb-2">
                    <strong>Parameters:</strong> {JSON.stringify(execution.parameters)}
                  </div>
                {/if}

                {#if execution.error}
                  <div class="text-xs text-red-600 mb-2">
                    <strong>Error:</strong> {execution.error}
                  </div>
                {/if}

                <div class="flex items-center justify-end">
                  <button
                    onclick={(e) => { e.stopPropagation(); copyToClipboard(JSON.stringify(execution.result, null, 2)); }}
                    class="text-xs text-gray-600 hover:text-gray-800 flex items-center"
                  >
                    <Copy size={12} class="mr-1" />
                    Copy Result
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>