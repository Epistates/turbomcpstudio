<script lang="ts">
  import { CheckCircle, XCircle, AlertCircle, Info, ChevronDown, ChevronRight } from 'lucide-svelte';
  import type { ValidationResult, SchemaValidationError } from '$lib/utils/elicitationSchemaValidator';
  import { getValidationSummary } from '$lib/utils/elicitationSchemaValidator';

  interface Props {
    result: ValidationResult;
    compact?: boolean;
  }

  const { result, compact = false }: Props = $props();

  let showErrors = $state(true);
  let showWarnings = $state(false);

  const errorCount = $derived(result.errors.length);
  const warningCount = $derived(result.warnings.length);
  const hasIssues = $derived(errorCount > 0 || warningCount > 0);

  function getSeverityIcon(severity: SchemaValidationError['severity']) {
    switch (severity) {
      case 'error': return XCircle;
      case 'warning': return AlertCircle;
      case 'info': return Info;
    }
  }

  function getSeverityColor(severity: SchemaValidationError['severity']) {
    switch (severity) {
      case 'error': return 'text-red-600 dark:text-red-400';
      case 'warning': return 'text-yellow-600 dark:text-yellow-400';
      case 'info': return 'text-blue-600 dark:text-blue-400';
    }
  }

  function getBorderColor(severity: SchemaValidationError['severity']) {
    switch (severity) {
      case 'error': return 'border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20';
      case 'warning': return 'border-yellow-200 dark:border-yellow-800 bg-yellow-50 dark:bg-yellow-900/20';
      case 'info': return 'border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20';
    }
  }
</script>

<div class="space-y-3">
  <!-- Summary -->
  <div class="border rounded-lg p-3 {result.valid ? 'border-green-200 dark:border-green-800 bg-green-50 dark:bg-green-900/20' : 'border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20'}">
    <div class="flex items-start gap-2">
      {#if result.valid && !hasIssues}
        <CheckCircle size={20} class="text-green-600 dark:text-green-400 flex-shrink-0 mt-0.5" />
        <div class="flex-1">
          <div class="text-sm font-medium text-green-800 dark:text-green-200">
            Schema is valid
          </div>
          {#if !compact}
            <div class="text-xs text-green-700 dark:text-green-300 mt-1">
              This schema conforms to MCP elicitation requirements
            </div>
          {/if}
        </div>
      {:else}
        <XCircle size={20} class="text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
        <div class="flex-1">
          <div class="text-sm font-medium text-red-800 dark:text-red-200">
            {getValidationSummary(result)}
          </div>
          {#if !compact && errorCount > 0}
            <div class="text-xs text-red-700 dark:text-red-300 mt-1">
              Fix errors to use this schema with MCP servers
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Errors -->
  {#if errorCount > 0}
    <div>
      <button
        onclick={() => showErrors = !showErrors}
        class="w-full flex items-center justify-between p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
      >
        <div class="flex items-center gap-2 text-sm font-medium text-red-800 dark:text-red-200">
          <XCircle size={16} />
          <span>Errors ({errorCount})</span>
        </div>
        {#if showErrors}
          <ChevronDown size={16} class="text-gray-400" />
        {:else}
          <ChevronRight size={16} class="text-gray-400" />
        {/if}
      </button>

      {#if showErrors}
        <div class="space-y-2 mt-2">
          {#each result.errors as error}
            {@const Icon = getSeverityIcon(error.severity)}
            <div class="border rounded-lg p-3 {getBorderColor(error.severity)}">
              <div class="flex items-start gap-2">
                <Icon size={16} class="{getSeverityColor(error.severity)} flex-shrink-0 mt-0.5" />
                <div class="flex-1 min-w-0">
                  <div class="text-xs font-medium {getSeverityColor(error.severity)} mb-1">
                    {error.field}
                  </div>
                  <div class="text-xs text-gray-700 dark:text-gray-300">
                    {error.error}
                  </div>
                  {#if error.suggestion}
                    <div class="text-xs text-gray-600 dark:text-gray-400 mt-1 italic">
                      💡 {error.suggestion}
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Warnings -->
  {#if warningCount > 0}
    <div>
      <button
        onclick={() => showWarnings = !showWarnings}
        class="w-full flex items-center justify-between p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
      >
        <div class="flex items-center gap-2 text-sm font-medium text-yellow-800 dark:text-yellow-200">
          <AlertCircle size={16} />
          <span>Warnings ({warningCount})</span>
        </div>
        {#if showWarnings}
          <ChevronDown size={16} class="text-gray-400" />
        {:else}
          <ChevronRight size={16} class="text-gray-400" />
        {/if}
      </button>

      {#if showWarnings}
        <div class="space-y-2 mt-2">
          {#each result.warnings as warning}
            {@const Icon = getSeverityIcon(warning.severity)}
            <div class="border rounded-lg p-3 {getBorderColor(warning.severity)}">
              <div class="flex items-start gap-2">
                <Icon size={16} class="{getSeverityColor(warning.severity)} flex-shrink-0 mt-0.5" />
                <div class="flex-1 min-w-0">
                  <div class="text-xs font-medium {getSeverityColor(warning.severity)} mb-1">
                    {warning.field}
                  </div>
                  <div class="text-xs text-gray-700 dark:text-gray-300">
                    {warning.error}
                  </div>
                  {#if warning.suggestion}
                    <div class="text-xs text-gray-600 dark:text-gray-400 mt-1 italic">
                      💡 {warning.suggestion}
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
