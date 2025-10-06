<!--
  PromptStepConfig - Composed component for configuring prompt operations in Collections

  Reuses prompt discovery logic from PromptDesigner
  Prompts are predefined templates with arguments
-->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import DynamicForm from '$lib/components/ui/DynamicForm.svelte';
  import type { ServerInfo } from '$lib/stores/serverStore';
  import type { PromptOperation } from '$lib/types/collections';
  import { AlertCircle, Loader } from 'lucide-svelte';

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

  interface Props {
    operation: PromptOperation;
    servers: ServerInfo[];
    availableVariables?: string[];
  }

  let { operation, servers, availableVariables = [] }: Props = $props();

  let availablePrompts = $state<PromptTemplate[]>([]);
  let selectedPromptDef = $state<PromptTemplate | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Load prompts from selected server
  async function loadPrompts() {
    if (!operation.server_alias) {
      availablePrompts = [];
      selectedPromptDef = null;
      return;
    }

    loading = true;
    error = null;

    try {
      const prompts = await invoke<PromptTemplate[]>('list_prompts', {
        serverId: operation.server_alias
      });

      availablePrompts = prompts;

      // If a prompt name is already set, find its definition
      if (operation.prompt_name) {
        selectedPromptDef = prompts.find(p => p.name === operation.prompt_name) || null;
      }
    } catch (err) {
      console.error('Failed to load prompts:', err);
      error = `Failed to load prompts: ${err}`;
      availablePrompts = [];
      selectedPromptDef = null;
    } finally {
      loading = false;
    }
  }

  // Load prompts when server changes
  $effect(() => {
    if (operation.server_alias) {
      loadPrompts();
    }
  });

  // Update prompt definition when prompt name changes
  $effect(() => {
    if (operation.prompt_name && availablePrompts.length > 0) {
      selectedPromptDef = availablePrompts.find(p => p.name === operation.prompt_name) || null;
    }
  });

  // Convert prompt arguments to JSON schema for DynamicForm
  function getPromptSchema() {
    if (!selectedPromptDef?.arguments) return null;

    const properties: Record<string, any> = {};
    const required: string[] = [];

    selectedPromptDef.arguments.forEach(arg => {
      properties[arg.name] = {
        type: arg.type || 'string',
        description: arg.description,
        default: arg.default
      };
      if (arg.required) {
        required.push(arg.name);
      }
    });

    return {
      type: 'object',
      properties,
      required
    };
  }

  // Handle parameter changes from DynamicForm
  function handleParametersChange(newParams: Record<string, any>) {
    operation.parameters = newParams;
  }
</script>

<div class="space-y-4">
  <!-- Server Selection -->
  <div>
    <label for="prompt-server" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
      Server
    </label>
    <select
      id="prompt-server"
      bind:value={operation.server_alias}
      class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
    >
      <option value="">Select a server...</option>
      {#each servers as server}
        <option value={server.id}>{server.config.name}</option>
      {/each}
    </select>
  </div>

  <!-- Prompt Selection (Dropdown from actual server prompts) -->
  {#if operation.server_alias}
    <div>
      <label for="prompt-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        Prompt Template
      </label>

      {#if loading}
        <div class="flex items-center gap-2 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
          <Loader size={16} class="animate-spin text-blue-600 dark:text-blue-400" />
          <span class="text-sm text-blue-700 dark:text-blue-300">Loading prompts...</span>
        </div>
      {:else if error}
        <div class="flex items-center gap-2 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
          <AlertCircle size={16} class="text-red-600 dark:text-red-400" />
          <span class="text-sm text-red-700 dark:text-red-300">{error}</span>
        </div>
      {:else if availablePrompts.length === 0}
        <div class="p-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
          <p class="text-sm text-gray-600 dark:text-gray-400">No prompts available on this server</p>
        </div>
      {:else}
        <select
          id="prompt-name"
          bind:value={operation.prompt_name}
          class="w-full p-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">Select a prompt...</option>
          {#each availablePrompts as prompt}
            <option value={prompt.name}>{prompt.title || prompt.name}</option>
          {/each}
        </select>

        {#if selectedPromptDef}
          <p class="mt-2 text-sm text-gray-600 dark:text-gray-400">
            {selectedPromptDef.description || 'No description available'}
          </p>
        {/if}
      {/if}
    </div>
  {/if}

  <!-- Prompt Arguments (DynamicForm - REUSED!) -->
  {#if selectedPromptDef && selectedPromptDef.arguments && selectedPromptDef.arguments.length > 0}
    {@const schema = getPromptSchema()}
    {#if schema}
      <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
          Arguments
        </label>

        <DynamicForm
          {schema}
          values={operation.parameters || {}}
          onValuesChange={handleParametersChange}
        />
      </div>
    {/if}
  {:else if operation.prompt_name && !loading}
    <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
      <p class="text-sm text-gray-500 dark:text-gray-400 italic">
        This prompt has no arguments
      </p>
    </div>
  {/if}

  <!-- Available variables hint -->
  {#if availableVariables.length > 0}
    <div class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
      <p class="text-sm text-blue-700 dark:text-blue-300 font-medium mb-2">
        Available Variables (for argument values):
      </p>
      <div class="flex flex-wrap gap-2">
        {#each availableVariables as varName}
          <code class="px-2 py-1 text-xs bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 rounded border border-blue-200 dark:border-blue-700">
            &#36;&#123;{varName}&#125;
          </code>
        {/each}
      </div>
    </div>
  {/if}
</div>
