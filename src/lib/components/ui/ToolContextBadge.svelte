<!--
  ToolContextBadge - Display MCP tool context for sampling requests

  Features:
  - Compact mode for lists
  - Detailed mode with expandable parameters
  - Handles both toolContext and operationContext
  - Dark mode support
-->
<script lang="ts">
  import { Zap, MessageSquare } from 'lucide-svelte';
  import type { ToolContext } from '$lib/types/sampling';

  const { toolContext, operationContext, variant = 'default', serverName } = $props<{
    toolContext?: ToolContext | null;
    operationContext?: string;
    variant?: 'compact' | 'default' | 'detailed';
    serverName?: string;
  }>();

  const hasContext = $derived(toolContext || operationContext);
</script>

{#if hasContext}
  {#if variant === 'compact'}
    <!-- Compact version for request lists -->
    <div class="p-2 bg-blue-50/50 border border-blue-200 rounded">
      {#if toolContext}
        <div class="flex items-center gap-1.5 mb-1">
          <Zap size={10} class="text-blue-600" />
          <span class="text-xs font-medium text-blue-800">Tool: {toolContext.toolName}</span>
        </div>
      {/if}
      {#if operationContext}
        <p class="text-xs text-gray-600">
          {operationContext}
        </p>
      {/if}
    </div>
  {:else if variant === 'detailed'}
    <!-- Detailed version with expandable parameters -->
    <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
      {#if toolContext}
        <div class="mb-3">
          <div class="flex items-center gap-2 mb-2">
            <Zap size={14} class="text-blue-600" />
            <span class="text-sm font-medium text-blue-800 dark:text-blue-200">
              Tool: {toolContext.toolName}
            </span>
          </div>
          <p class="text-sm text-blue-700 dark:text-blue-300 mb-2">
            {operationContext || 'Server tool execution requesting LLM assistance'}
          </p>
          <details class="text-xs">
            <summary class="cursor-pointer text-blue-600 hover:text-blue-800 font-medium mb-1"
              >View Tool Parameters</summary
            >
            <pre
              class="mt-1 p-2 bg-white dark:bg-blue-900/50 border border-blue-200 dark:border-blue-700 rounded text-blue-800 dark:text-blue-200 overflow-x-auto">{JSON.stringify(toolContext.parameters, null, 2)}</pre>
          </details>
        </div>
      {:else if operationContext}
        <div class="flex items-start gap-2">
          <MessageSquare size={14} class="text-blue-600 mt-0.5" />
          <div>
            <p class="text-sm font-medium text-blue-800 dark:text-blue-200 mb-1">Server Operation</p>
            <p class="text-sm text-blue-700 dark:text-blue-300">
              {operationContext}
            </p>
          </div>
        </div>
      {/if}
      {#if serverName}
        <div class="mt-3 text-xs text-blue-600 dark:text-blue-400">
          💡 This is a request from <strong>{serverName}</strong> needing LLM assistance
        </div>
      {/if}
    </div>
  {:else}
    <!-- Default version -->
    <div class="p-3 bg-blue-50 border border-blue-200 rounded dark:bg-blue-900/20 dark:border-blue-800">
      {#if toolContext}
        <div class="flex items-center gap-2 mb-1">
          <Zap size={14} class="text-blue-600" />
          <span class="text-sm font-medium text-blue-800 dark:text-blue-200">
            Tool: {toolContext.toolName}
          </span>
        </div>
      {/if}
      {#if operationContext}
        <p class="text-sm text-gray-600 dark:text-gray-400">
          {operationContext}
        </p>
      {/if}
    </div>
  {/if}
{/if}
