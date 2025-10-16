<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { serverStore, type ServerInfo, type ToolDefinition, type ToolExecution } from '$lib/stores/serverStore';
  import { contextStore } from '$lib/stores/contextStore';
  import { uiStore } from '$lib/stores/uiStore';
  import DynamicForm from '$lib/components/ui/DynamicForm.svelte';
  import JsonViewer from '$lib/components/ui/JsonViewer.svelte';
  import { validateToolParameters, generateDefaultParameters } from '$lib/utils/schemaValidation';
  import { createLogger } from '$lib/utils/logger';

  // Initialize scoped logger
  const logger = createLogger('ToolExplorer');

  interface ValidationResult {
    isValid: boolean;
    errors: Record<string, string[]>;
  }

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
    AlertTriangle,
    Bookmark
  } from 'lucide-svelte';

  // ✅ Use contextStore for server selection (no local state, no manual sync)
  const context = $derived($contextStore);
  const selectedServer = $derived(context.selectedServer);
  const selectedServerId = $derived(context.selectedServerId);

  // Component state
  let tools: ToolDefinition[] = $state([]);
  let loading = $state(false);
  let loadingPromise: Promise<void> | null = null; // Track in-flight load
  let searchQuery = $state('');
  let jsonSearchTerm = $state('');
  let selectedTool: ToolDefinition | null = $state(null);
  let toolParameters = $state<Record<string, any>>({});
  let toolResult: any = $state(null);
  let expandedTool: string | null = $state(null);
  let executing = $state(false);
  let isHistoricalResult = $state(false);
  let validationResult: ValidationResult = $state({ isValid: true, errors: {} });
  let showValidationErrors = $state(false);
  let persistedToolSelection: { name: string; serverId: string } | undefined = $state(undefined);

  // ✅ Subscribe to stores for execution history and persisted selection
  const serverStoreState = $derived($serverStore);
  const executionHistory = $derived(() => {
    const toolName = selectedTool?.name;
    if (toolName) {
      // Filter by specific tool when one is selected
      return selectedServerId
        ? serverStoreState.toolExecutions.filter((e: any) =>
            e.serverId === selectedServerId &&
            e.tool === toolName &&
            !e.tool.startsWith('prompt:')
          )
        : serverStoreState.toolExecutions.filter((e: any) =>
            e.tool === toolName &&
            !e.tool.startsWith('prompt:')
          );
    } else {
      // Show all history when no tool is selected
      return selectedServerId
        ? serverStoreState.toolExecutions.filter((e: any) =>
            e.serverId === selectedServerId &&
            !e.tool.startsWith('prompt:')
          )
        : serverStoreState.toolExecutions.filter((e: any) =>
            !e.tool.startsWith('prompt:')
          );
    }
  });

  // Subscribe to persisted tool selection from uiStore
  $effect(() => {
    const unsubscribe = uiStore.subscribe((state: any) => {
      persistedToolSelection = state.selectedTool;
    });
    return unsubscribe;
  });

  // Load tools when server selection changes
  // Don't track executing here - it causes issues with sampling/elicitation
  $effect(() => {
    if (selectedServerId) {
      logger.debug(`[loadTools effect] Server changed to: ${selectedServerId}`);
      loadTools();
    }
  });

  // Restore persisted tool selection when tools are loaded
  $effect(() => {
    if (tools.length > 0 && persistedToolSelection && persistedToolSelection.serverId && selectedServerId === persistedToolSelection.serverId) {
      const persistedTool = tools.find(t => t.name === persistedToolSelection!.name);
      if (persistedTool && (!selectedTool || selectedTool.name !== persistedTool.name)) {
        selectedTool = persistedTool;
        toolParameters = persistedTool.input_schema ? generateDefaultParameters(persistedTool.input_schema) : {};
      }
    }
  });

  // Note: Execution history is now updated in the subscription above whenever selectedTool or store state changes

  const filteredTools = $derived(
    tools.filter(tool => 
      !searchQuery || 
      tool.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (tool.description && tool.description.toLowerCase().includes(searchQuery.toLowerCase()))
    )
  );

  async function loadTools() {
    if (!selectedServerId) return;

    // ✅ Prevent concurrent loadTools() calls
    if (loadingPromise) {
      logger.debug('Load already in progress, waiting for existing promise');
      return loadingPromise;
    }

    loading = true;
    logger.debug('Loading tools for server:', selectedServerId);

    // Create and track the loading promise
    loadingPromise = (async () => {
      try {
        // ✅ FIX: Trust TurboMCP to handle concurrent requests properly
        // No timeout - let the backend manage request queueing
        tools = await serverStore.listTools(selectedServerId);
        logger.debug(`✅ Loaded ${tools.length} tools successfully`);
      } catch (error) {
        logger.error('❌ Failed to load tools:', error);
        uiStore.showError(`Failed to load tools: ${error}`);
        tools = [];
      } finally {
        loading = false;
        loadingPromise = null; // Clear the promise
        logger.debug('Loading state reset to false');
      }
    })();

    return loadingPromise;
  }


  function selectTool(tool: ToolDefinition) {
    selectedTool = tool;
    toolResult = null;
    expandedTool = null;
    isHistoricalResult = false;
    showValidationErrors = false;
    validationResult = { isValid: true, errors: {} };

    // Persist tool selection to UI store
    if (selectedServerId) {
      uiStore.setSelectedTool(tool.name, selectedServerId);
    }

    // Initialize parameters with schema defaults
    toolParameters = tool.input_schema ? generateDefaultParameters(tool.input_schema) : {};
  }

  function clearSelectedTool() {
    selectedTool = null;
    toolResult = null;
    expandedTool = null;
    isHistoricalResult = false;
    toolParameters = {};
    uiStore.clearSelectedTool();
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

    // Clean up parameters - remove empty strings for optional fields
    const cleanedParameters: Record<string, any> = {};
    const requiredFields = selectedTool.input_schema?.required || [];

    Object.entries(toolParameters).forEach(([key, value]) => {
      // Always include required fields (even if empty - let server validation handle it)
      // For optional fields, only include non-empty values
      if (requiredFields.includes(key) || (value !== '' && value !== null && value !== undefined)) {
        cleanedParameters[key] = value;
      }
    });

    logger.debug(`🚀 Setting executing = true (calling tool: ${selectedTool.name})`);
    executing = true;
    try {
      logger.debug(`📞 Calling serverStore.callTool for: ${selectedTool.name}`);
      const result = await serverStore.callTool(
        selectedServerId,
        selectedTool.name,
        cleanedParameters
      );

      logger.debug(`✅ Tool ${selectedTool.name} completed successfully`);
      toolResult = result;
      isHistoricalResult = false;

      uiStore.showSuccess(`Tool "${selectedTool.name}" executed successfully`);
    } catch (error) {
      logger.error(`❌ Tool ${selectedTool.name} execution failed:`, error);
      const errorResult = { error: String(error) };
      toolResult = errorResult;
      isHistoricalResult = false;
      uiStore.showError(`Tool execution failed: ${error}`);
    } finally {
      logger.debug(`🏁 Setting executing = false (tool: ${selectedTool.name} finished)`);
      executing = false;

      // ✅ FIX P2: Ensure tools list remains accessible after execution
      // Reset loading state to prevent stuck UI (especially after sampling/elicitation)
      loading = false;

      logger.debug('Tool execution complete, state reset');
    }
  }

  function toggleToolExpansion(toolName: string) {
    expandedTool = expandedTool === toolName ? null : toolName;
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    uiStore.showSuccess('Copied to clipboard');
  }

  function rerunFromHistory(historyItem: ToolExecution) {
    const tool = tools.find(t => t.name === historyItem.tool);
    if (tool) {
      selectTool(tool);
      toolParameters = { ...historyItem.parameters };
      // Also show the previous result - INCLUDE ERROR if present
      if (historyItem.status === 'error' && historyItem.error) {
        // For error executions, create result object with error
        toolResult = { error: historyItem.error };
      } else {
        // For successful executions, use the result
        toolResult = historyItem.result;
      }
      isHistoricalResult = true;
    }
  }

  // NEW: Save current tool configuration to a collection
  async function saveToCollection() {
    if (!selectedTool || !selectedServerId) return;

    try {
      // Create a new collection with this tool as first step
      const newCollection = {
        id: crypto.randomUUID(),
        name: `${selectedTool.name} Test`,
        description: `Test collection for ${selectedTool.name}`,
        tags: ['tool-test'],
        workflow: [
          {
            id: crypto.randomUUID(),
            name: `Call ${selectedTool.name}`,
            description: selectedTool.description || '',
            enabled: true,
            continue_on_error: false,
            timeout_ms: 30000,
            depends_on: [],
            execution_order: 0,
            operation: {
              type: 'tool',
              server_alias: selectedServerId,
              tool_name: selectedTool.name,
              parameters: { ...toolParameters }
            },
            variable_extracts: [],
            assertions: []
          }
        ],
        variables: {},
        environment: {
          name: 'Default',
          description: 'Default environment',
          servers: {
            [selectedServerId]: selectedServerId
          },
          variables: {}
        },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        version: '1.0.0',
        run_count: 0
      };

      // Save to database via invoke
      await invoke('save_collection', { collection: newCollection });

      uiStore.showSuccess(`Saved "${selectedTool.name}" to new collection`);
      uiStore.setView('collections');
    } catch (error) {
      logger.error('Failed to save to collection:', error);
      uiStore.showError(`Failed to save to collection: ${error}`);
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

      <!-- Search -->
      <div class="relative">
        <Search size={16} class="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 dark:text-gray-500" />
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search tools..."
          class="form-input has-icon-left"
        />
      </div>
    </div>

    <!-- Tool List -->
    <div class="flex-1 overflow-y-auto">
      {#if !selectedServer}
        <div class="p-4 text-center">
          <Database size={32} class="mx-auto text-gray-400 dark:text-gray-500 mb-2" />
          <p class="text-sm text-gray-600 dark:text-gray-400">No server selected</p>
          <p class="text-xs text-gray-500 dark:text-gray-500">Use the server selector above to choose a server with tools capability</p>
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
                <div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                  <h4 class="font-medium text-gray-700 dark:text-gray-300 mb-2 text-xs">Input Schema</h4>
                  <JsonViewer
                    data={tool.input_schema}
                    expanded={false}
                    showCopy={true}
                    maxHeight="300px"
                  />
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
          {#if context.availableServers.length === 0}
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
                  <span>{Object.keys(validationResult.errors).length} validation error{Object.keys(validationResult.errors).length > 1 ? 's' : ''}</span>
                </div>
              {/if}

              <!-- NEW: Save to Collection button (shows after execution) -->
              {#if toolResult && !isHistoricalResult}
                <button
                  onclick={saveToCollection}
                  class="btn-secondary flex items-center gap-2"
                  title="Save this tool configuration to a new collection"
                >
                  <Bookmark size={16} />
                  Save to Collection
                </button>
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
          <div class="min-w-0 flex-1 flex flex-col bg-white dark:bg-gray-800">
            <div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between flex-shrink-0">
              <div class="flex items-center gap-2">
                <h3 class="font-medium text-gray-900 dark:text-gray-100">
                  Result
                </h3>
                {#if isHistoricalResult}
                  <span class="text-xs bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 px-2 py-1 rounded-full">Historical</span>
                {/if}
              </div>
              {#if toolResult}
                <div class="flex items-center gap-2">
                  <div class="relative">
                    <Search size={14} class="absolute left-2 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500" />
                    <input
                      type="text"
                      bind:value={jsonSearchTerm}
                      placeholder="Search JSON..."
                      class="pl-7 pr-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
                    />
                  </div>
                  <button
                    onclick={() => copyToClipboard(JSON.stringify(toolResult, null, 2))}
                    class="px-3 py-1.5 text-sm flex items-center gap-1.5 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-md transition-colors"
                    title="Copy JSON"
                  >
                    <Copy size={14} />
                    Copy
                  </button>
                </div>
              {/if}
            </div>

            <div class="flex-1 overflow-hidden min-h-0">
              {#if toolResult}
                <JsonViewer
                  data={toolResult}
                  expanded={true}
                  embedded={true}
                  bind:searchTerm={jsonSearchTerm}
                />
              {:else}
                <div class="flex items-center justify-center h-full">
                  <div class="text-center">
                    <CheckCircle size={32} class="mx-auto text-gray-400 dark:text-gray-500 mb-2" />
                    <p class="text-sm text-gray-600 dark:text-gray-400">Execute the tool to see results</p>
                  </div>
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- Execution History -->
        {#if executionHistory().length > 0}
          <div class="border-t border-gray-200">
            <div class="p-4">
              <div class="flex items-center justify-between mb-3">
                <h3 class="font-medium text-gray-900 flex items-center">
                  <History size={16} class="mr-2" />
                  Recent Executions
                </h3>
                <span class="text-xs text-gray-500">{executionHistory().length} executions</span>
              </div>

              <div class="space-y-2 max-h-48 overflow-y-auto">
                {#each executionHistory().slice(0, 5) as item}
                  <button
                    onclick={() => rerunFromHistory(item)}
                    class="w-full p-3 text-left bg-gray-50 dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 rounded border border-gray-200 dark:border-gray-700 transition-colors"
                  >
                    <div class="flex items-center justify-between mb-1.5">
                      <div class="flex items-center space-x-2">
                        {#if item.status === 'success'}
                          <CheckCircle size={12} class="text-green-600 dark:text-green-500" />
                        {:else}
                          <AlertCircle size={12} class="text-red-600 dark:text-red-500" />
                        {/if}
                        <span class="text-sm font-medium text-gray-900 dark:text-gray-100">{item.tool}</span>
                      </div>
                      <div class="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
                        {#if item.duration}
                          <span class="flex items-center gap-1">
                            <Clock size={10} />
                            {item.duration}ms
                          </span>
                        {/if}
                        <span>{new Date(item.timestamp).toLocaleTimeString()}</span>
                      </div>
                    </div>
                    {#if Object.keys(item.parameters).length > 0}
                      <div class="mt-1.5 pl-5">
                        <div class="text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5">
                          {#each Object.entries(item.parameters).slice(0, 3) as [key, value]}
                            <div class="flex items-start gap-1.5">
                              <span class="text-gray-500 dark:text-gray-500 shrink-0">{key}:</span>
                              <span class="truncate text-gray-700 dark:text-gray-300">
                                {typeof value === 'string' ? value : JSON.stringify(value)}
                              </span>
                            </div>
                          {/each}
                          {#if Object.keys(item.parameters).length > 3}
                            <div class="text-gray-400 dark:text-gray-500 italic">
                              +{Object.keys(item.parameters).length - 3} more...
                            </div>
                          {/if}
                        </div>
                      </div>
                    {:else}
                      <p class="text-xs text-gray-500 dark:text-gray-400 pl-5 italic">No parameters</p>
                    {/if}
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
