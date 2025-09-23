<script lang="ts">
  import { onMount } from 'svelte';
  import { serverStore, type ServerInfo, type ToolDefinition, type ToolExecution } from '$lib/stores/serverStore';
  import { uiStore } from '$lib/stores/uiStore';
  import DynamicForm from '$lib/components/ui/DynamicForm.svelte';
  import { validateToolParameters, generateDefaultParameters, type ValidationResult } from '$lib/utils/schemaValidation';
  import {
    Zap,
    Play,
    RefreshCw,
    FileCode,
    Search,
    AlertCircle,
    CheckCircle,
    Clock,
    Database,
    ChevronDown,
    ChevronRight,
    Copy,
    History,
    AlertTriangle
  } from 'lucide-svelte';

  let servers: ServerInfo[] = $state([]);
  let selectedServerId: string | undefined = $state(undefined);
  let tools: ToolDefinition[] = $state([]);
  let loading = $state(false);
  let searchQuery = $state('');
  let selectedTool: ToolDefinition | null = $state(null);
  let toolParameters = $state<Record<string, any>>({});
  let toolResult: any = $state(null);
  let executionHistory: ToolExecution[] = $state([]);
  let expandedTool: string | null = $state(null);
  let executing = $state(false);
  let isHistoricalResult = $state(false);
  let validationResult: ValidationResult = $state({ isValid: true, errors: [] });
  let showValidationErrors = $state(false);

  // Subscribe to stores
  $effect(() => {
    const unsubscribeServers = serverStore.subscribe(state => {
      const connectedServers = state.servers.filter(s => s.status?.toLowerCase() === 'connected');
      servers = connectedServers;

      // Only update selectedServerId if it's different to avoid loops
      if (selectedServerId !== state.selectedServerId) {
        selectedServerId = state.selectedServerId;
      }

      // Auto-select first connected server if none selected (only once)
      if (!state.selectedServerId && connectedServers.length > 0 && !selectedServerId) {
        serverStore.selectServer(connectedServers[0].id);
      }

      // Update execution history from store - filter by both server and selected tool
      executionHistory = selectedServerId
        ? state.toolExecutions.filter(e =>
            e.serverId === selectedServerId &&
            (!selectedTool || e.tool === selectedTool.name) &&
            !e.tool.startsWith('prompt:') // Exclude prompt executions
          )
        : state.toolExecutions.filter(e =>
            (!selectedTool || e.tool === selectedTool.name) &&
            !e.tool.startsWith('prompt:') // Exclude prompt executions
          );
    });

    return () => {
      unsubscribeServers();
    };
  });

  // Load tools when server selection changes
  $effect(() => {
    if (selectedServerId) {
      loadTools();
    }
  });

  // Note: Execution history is now updated in the subscription above whenever selectedTool or store state changes

  const selectedServer = $derived(
    servers.find(s => s.id === selectedServerId)
  );

  const filteredTools = $derived(
    tools.filter(tool => 
      !searchQuery || 
      tool.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (tool.description && tool.description.toLowerCase().includes(searchQuery.toLowerCase()))
    )
  );

  async function loadTools() {
    if (!selectedServerId) return;
    
    loading = true;
    try {
      tools = await serverStore.listTools(selectedServerId);
    } catch (error) {
      console.error('Failed to load tools:', error);
      uiStore.showError(`Failed to load tools: ${error}`);
      tools = [];
    } finally {
      loading = false;
    }
  }

  function selectServer(serverId: string) {
    serverStore.selectServer(serverId);
    selectedServerId = serverId;
  }

  function selectTool(tool: ToolDefinition) {
    selectedTool = tool;
    toolResult = null;
    expandedTool = null;
    isHistoricalResult = false;
    showValidationErrors = false;
    validationResult = { isValid: true, errors: [] };

    // Initialize parameters with schema defaults
    toolParameters = tool.input_schema ? generateDefaultParameters(tool.input_schema) : {};
  }

  function handleParameterChange(newParams: Record<string, any>) {
    toolParameters = newParams;
  }

  function handleValidationChange(validation: ValidationResult) {
    validationResult = validation;
  }

  async function executeTool() {
    if (!selectedTool || !selectedServerId) return;

    // Show validation errors if form is invalid
    if (!validationResult.isValid) {
      showValidationErrors = true;
      uiStore.showError('Please fix validation errors before executing the tool');
      return;
    }

    executing = true;
    try {
      const result = await serverStore.callTool(
        selectedServerId,
        selectedTool.name,
        toolParameters
      );

      toolResult = result;
      isHistoricalResult = false;

      uiStore.showSuccess(`Tool "${selectedTool.name}" executed successfully`);
    } catch (error) {
      console.error('Tool execution failed:', error);
      const errorResult = { error: String(error) };
      toolResult = errorResult;

      uiStore.showError(`Tool execution failed: ${error}`);
    } finally {
      executing = false;
    }
  }

  function toggleToolExpansion(toolName: string) {
    expandedTool = expandedTool === toolName ? null : toolName;
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    uiStore.showSuccess('Copied to clipboard');
  }

  function formatJson(obj: any): string {
    return JSON.stringify(obj, null, 2);
  }

  function rerunFromHistory(historyItem: ToolExecution) {
    const tool = tools.find(t => t.name === historyItem.tool);
    if (tool) {
      selectTool(tool);
      toolParameters = { ...historyItem.parameters };
      // Also show the previous result
      toolResult = historyItem.result;
      isHistoricalResult = true;
    }
  }
</script>

<div class="h-full flex bg-gray-50 dark:bg-gray-900">
  <!-- Left Panel: Tool List -->
  <div class="w-80 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col">
    <!-- Header -->
    <div class="p-4 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">Tools</h2>
        <button
          onclick={loadTools}
          disabled={loading || !selectedServerId}
          class="p-2 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg {loading ? 'animate-spin' : ''}"
        >
          <RefreshCw size={16} />
        </button>
      </div>

      <!-- Server Selection -->
      {#if servers.length > 0}
        <div class="mb-4">
          <label class="form-label" for="server-select">Server</label>
          <select
            bind:value={selectedServerId}
            onchange={(e) => selectServer((e.target as HTMLSelectElement).value)}
            class="form-input"
            id="server-select"
          >
            {#each servers as server}
              <option value={server.id}>{server.config.name || 'Unnamed Server'}</option>
            {/each}
          </select>
        </div>
      {/if}

      <!-- Search -->
      <div class="relative">
        <Search size={16} class="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 dark:text-gray-500" />
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search tools..."
          class="form-input pl-10"
        />
      </div>
    </div>

    <!-- Tool List -->
    <div class="flex-1 overflow-y-auto">
      {#if !selectedServer}
        <div class="p-4 text-center">
          <Database size={32} class="mx-auto text-gray-400 dark:text-gray-500 mb-2" />
          <p class="text-sm text-gray-600 dark:text-gray-400">No connected servers</p>
          <p class="text-xs text-gray-500 dark:text-gray-500">Connect to an MCP server to see available tools</p>
        </div>
      {:else if loading}
        <div class="p-4 text-center">
          <RefreshCw size={24} class="mx-auto text-gray-400 dark:text-gray-500 animate-spin mb-2" />
          <p class="text-sm text-gray-600 dark:text-gray-400">Loading tools...</p>
        </div>
      {:else if filteredTools.length === 0}
        <div class="p-4 text-center">
          <Zap size={32} class="mx-auto text-gray-400 dark:text-gray-500 mb-2" />
          <p class="text-sm text-gray-600 dark:text-gray-400">
            {searchQuery ? 'No tools match your search' : 'No tools available'}
          </p>
          {#if !searchQuery && tools.length === 0}
            <p class="text-xs text-gray-500 dark:text-gray-500 mt-1">
              This server doesn't expose any tools
            </p>
          {/if}
        </div>
      {:else}
        <div class="p-2">
          {#each filteredTools as tool}
            <button
              onclick={() => selectTool(tool)}
              class="w-full p-3 text-left rounded-lg border border-transparent hover:border-mcp-primary-200 hover:bg-mcp-primary-50 dark:hover:border-mcp-primary-600 dark:hover:bg-mcp-primary-900/20 transition-colors mb-2 {selectedTool?.name === tool.name ? 'border-mcp-primary-300 bg-mcp-primary-50 dark:border-mcp-primary-600 dark:bg-mcp-primary-900/30' : ''}"
            >
              <div class="flex items-start justify-between">
                <div class="flex-1 min-w-0">
                  <h3 class="font-medium text-gray-900 dark:text-gray-100 truncate">{tool.name}</h3>
                  {#if tool.description}
                    <p class="text-xs text-gray-600 dark:text-gray-400 mt-1 line-clamp-2">{tool.description}</p>
                  {/if}

                  <!-- Schema info -->
                  {#if tool.input_schema?.properties}
                    <div class="flex items-center mt-2 text-xs text-gray-500 dark:text-gray-400">
                      <FileCode size={12} class="mr-1" />
                      {Object.keys(tool.input_schema.properties).length} parameters
                      {#if tool.input_schema.required?.length}
                        • {tool.input_schema.required.length} required
                      {/if}
                    </div>
                  {/if}
                </div>
                
                <span
                  onclick={(e) => { e.stopPropagation(); toggleToolExpansion(tool.name); }}
                  onkeydown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      e.stopPropagation();
                      toggleToolExpansion(tool.name);
                    }
                  }}
                  class="p-1 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 cursor-pointer"
                  role="button"
                  tabindex="0"
                  aria-label={expandedTool === tool.name ? 'Collapse tool details' : 'Expand tool details'}
                >
                  {#if expandedTool === tool.name}
                    <ChevronDown size={14} />
                  {:else}
                    <ChevronRight size={14} />
                  {/if}
                </span>
              </div>
              
              <!-- Expanded schema view -->
              {#if expandedTool === tool.name}
                <div class="mt-3 pt-3 border-t border-gray-200">
                  <div class="text-xs">
                    <h4 class="font-medium text-gray-700 mb-2">Input Schema</h4>
                    <pre class="bg-gray-100 p-2 rounded text-xs overflow-x-auto">{formatJson(tool.input_schema)}</pre>
                  </div>
                </div>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Right Panel: Tool Execution -->
  <div class="flex-1 flex flex-col">
    {#if !selectedTool}
      <!-- Welcome state -->
      <div class="flex-1 flex items-center justify-center">
        <div class="text-center max-w-md">
          <Zap size={48} class="mx-auto text-gray-400 dark:text-gray-500 mb-4" />
          <h3 class="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">Select a Tool</h3>
          <p class="text-gray-600 dark:text-gray-400 mb-4">
            Choose a tool from the list to see its parameters and execute it
          </p>
          {#if servers.length === 0}
            <button
              onclick={() => uiStore.openModal('addServer')}
              class="btn-primary"
            >
              Connect to Server
            </button>
          {/if}
        </div>
      </div>
    {:else}
      <!-- Tool execution interface -->
      <div class="h-full flex flex-col">
        <!-- Header -->
        <div class="p-6 border-b border-gray-200 dark:border-gray-700">
          <div class="flex items-start justify-between">
            <div>
              <h2 class="text-xl font-semibold text-gray-900 dark:text-gray-100">{selectedTool.name}</h2>
              {#if selectedTool.description}
                <p class="text-gray-600 dark:text-gray-400 mt-1">{selectedTool.description}</p>
              {/if}
              <div class="flex items-center mt-2 text-sm text-gray-500 dark:text-gray-400">
                <Database size={14} class="mr-1" />
                {selectedServer?.config.name || 'Unnamed Server'}
              </div>
            </div>
            
            <div class="flex items-center gap-3">
              {#if !validationResult.isValid && showValidationErrors}
                <div class="flex items-center gap-1 text-sm text-red-600">
                  <AlertTriangle size={14} />
                  <span>{validationResult.errors.length} validation error{validationResult.errors.length > 1 ? 's' : ''}</span>
                </div>
              {/if}

              <button
                onclick={executeTool}
                disabled={executing || !selectedServerId || (!validationResult.isValid && showValidationErrors)}
                class="btn-primary {executing || (!validationResult.isValid && showValidationErrors) ? 'opacity-50' : ''}"
              >
                {#if executing}
                  <RefreshCw size={16} class="mr-2 animate-spin" />
                  Executing...
                {:else}
                  <Play size={16} class="mr-2" />
                  Execute
                {/if}
              </button>
            </div>
          </div>
        </div>

        <div class="flex-1 flex overflow-hidden min-h-0">
          <!-- Parameters Panel -->
          <div class="min-w-0 flex-1 border-r border-gray-200 flex flex-col">
            <div class="p-4 border-b border-gray-200 flex-shrink-0">
              <h3 class="font-medium text-gray-900">Parameters</h3>
            </div>

            <div class="flex-1 overflow-y-auto p-4 min-h-0">
              {#if selectedTool.input_schema}
                <DynamicForm
                  schema={selectedTool.input_schema}
                  values={toolParameters}
                  onValuesChange={handleParameterChange}
                  onValidationChange={handleValidationChange}
                  disabled={executing}
                />
              {:else}
                <div class="text-center py-8">
                  <FileCode size={32} class="mx-auto text-gray-400 mb-2" />
                  <p class="text-sm text-gray-600">No parameters required</p>
                </div>
              {/if}
            </div>
          </div>

          <!-- Results Panel -->
          <div class="min-w-0 flex-1 flex flex-col">
            <div class="p-4 border-b border-gray-200 flex items-center justify-between flex-shrink-0">
              <h3 class="font-medium text-gray-900 flex items-center gap-2">
                Result
                {#if isHistoricalResult}
                  <span class="text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded-full">Historical</span>
                {/if}
              </h3>
              {#if toolResult}
                <button
                  onclick={() => copyToClipboard(formatJson(toolResult))}
                  class="p-1 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded"
                  title="Copy result to clipboard"
                >
                  <Copy size={14} />
                </button>
              {/if}
            </div>

            <div class="flex-1 overflow-y-auto p-4 min-h-0">
              {#if toolResult}
                <div class="bg-gray-900 text-gray-100 p-4 rounded-lg font-mono text-sm overflow-x-auto">
                  <pre>{formatJson(toolResult)}</pre>
                </div>
              {:else}
                <div class="text-center py-8">
                  <CheckCircle size={32} class="mx-auto text-gray-400 mb-2" />
                  <p class="text-sm text-gray-600">Execute the tool to see results</p>
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- Execution History -->
        {#if executionHistory.length > 0}
          <div class="border-t border-gray-200">
            <div class="p-4">
              <div class="flex items-center justify-between mb-3">
                <h3 class="font-medium text-gray-900 flex items-center">
                  <History size={16} class="mr-2" />
                  Recent Executions
                </h3>
                <span class="text-xs text-gray-500">{executionHistory.length} executions</span>
              </div>
              
              <div class="space-y-2 max-h-32 overflow-y-auto">
                {#each executionHistory.slice(0, 5) as item}
                  <button
                    onclick={() => rerunFromHistory(item)}
                    class="w-full p-2 text-left bg-gray-50 dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 rounded border border-gray-200 dark:border-gray-700 transition-colors"
                  >
                    <div class="flex items-center justify-between">
                      <div class="flex items-center space-x-2">
                        {#if item.status === 'success'}
                          <CheckCircle size={12} class="text-green-600" />
                        {:else}
                          <AlertCircle size={12} class="text-red-600" />
                        {/if}
                        <span class="text-sm font-medium text-gray-900 dark:text-gray-100">{item.tool}</span>
                      </div>
                      <span class="text-xs text-gray-500 dark:text-gray-400">
                        {new Date(item.timestamp).toLocaleTimeString()}
                      </span>
                    </div>
                    <p class="text-xs text-gray-600 dark:text-gray-400 mt-1 truncate">
                      {Object.keys(item.parameters).length} parameters
                      {#if item.duration}
                        • {item.duration}ms
                      {/if}
                    </p>
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>