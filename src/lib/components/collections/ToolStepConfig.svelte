<!--
  ToolStepConfig - Composed component for configuring tool operations in Collections

  Reuses existing components:
  - DynamicForm for parameter validation (from ToolExplorer)
  - Tool discovery logic (invoke list_tools)
  - Server selection

  This component COMPOSES existing functionality instead of reimplementing it.
-->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createLogger } from '$lib/utils/logger';
  import DynamicForm from '$lib/components/ui/DynamicForm.svelte';
  import type { ServerInfo, ToolDefinition } from '$lib/stores/serverStore';
  import type { ToolOperation } from '$lib/types/collections';
  import { AlertCircle, Loader } from 'lucide-svelte';

  // Initialize scoped logger
  const logger = createLogger('ToolStepConfig');

  interface Props {
    operation: ToolOperation;
    servers: ServerInfo[];
  }

  let { operation, servers }: Props = $props();

  let availableTools = $state<ToolDefinition[]>([]);
  let selectedToolDef = $state<ToolDefinition | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Load tools from selected server
  async function loadTools() {
    if (!operation.server_alias) {
      availableTools = [];
      selectedToolDef = null;
      return;
    }

    loading = true;
    error = null;

    try {
      const tools = await invoke<ToolDefinition[]>('list_tools', {
        serverId: operation.server_alias
      });

      availableTools = tools;

      // If a tool name is already set, find its definition
      if (operation.tool_name) {
        selectedToolDef = tools.find(t => t.name === operation.tool_name) || null;
      }
    } catch (err) {
      logger.error('Failed to load tools:', err);
      error = `Failed to load tools: ${err}`;
      availableTools = [];
      selectedToolDef = null;
    } finally {
      loading = false;
    }
  }

  // Load tools when server changes
  $effect(() => {
    if (operation.server_alias) {
      loadTools();
    }
  });

  // Update tool definition when tool name changes
  $effect(() => {
    if (operation.tool_name && availableTools.length > 0) {
      selectedToolDef = availableTools.find(t => t.name === operation.tool_name) || null;
    }
  });

  // Handle parameter changes from DynamicForm
  function handleParametersChange(newParams: Record<string, any>) {
    operation.parameters = newParams;
  }
</script>

<div class="space-y-4">
  <!-- Server Selection -->
  <div>
    <label for="tool-server" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
      Server
    </label>
    <select
      id="tool-server"
      bind:value={operation.server_alias}
      class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
    >
      <option value="">Select a server...</option>
      {#each servers as server}
        <option value={server.id}>{server.config.name}</option>
      {/each}
    </select>
  </div>

  <!-- Tool Selection (Dropdown from actual server tools) -->
  {#if operation.server_alias}
    <div>
      <label for="tool-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        Tool
      </label>

      {#if loading}
        <div class="flex items-center gap-2 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
          <Loader size={16} class="animate-spin text-blue-600 dark:text-blue-400" />
          <span class="text-sm text-blue-700 dark:text-blue-300">Loading tools...</span>
        </div>
      {:else if error}
        <div class="flex items-center gap-2 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
          <AlertCircle size={16} class="text-red-600 dark:text-red-400" />
          <span class="text-sm text-red-700 dark:text-red-300">{error}</span>
        </div>
      {:else if availableTools.length === 0}
        <div class="p-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
          <p class="text-sm text-gray-600 dark:text-gray-400">No tools available on this server</p>
        </div>
      {:else}
        <select
          id="tool-name"
          bind:value={operation.tool_name}
          class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">Select a tool...</option>
          {#each availableTools as tool}
            <option value={tool.name}>{tool.name}</option>
          {/each}
        </select>

        {#if selectedToolDef}
          <p class="mt-2 text-sm text-gray-600 dark:text-gray-400">
            {selectedToolDef.description || 'No description available'}
          </p>
        {/if}
      {/if}
    </div>
  {/if}

  <!-- Tool Parameters (DynamicForm - REUSED!) -->
  {#if selectedToolDef && selectedToolDef.input_schema}
    <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
      <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
        Parameters
      </label>

      <DynamicForm
        schema={selectedToolDef.input_schema}
        values={operation.parameters || {}}
        onValuesChange={handleParametersChange}
      />
    </div>
  {:else if operation.tool_name && !loading}
    <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
      <p class="text-sm text-gray-500 dark:text-gray-400 italic">
        No schema available for this tool
      </p>
    </div>
  {/if}
</div>
