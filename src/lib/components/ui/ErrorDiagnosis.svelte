<script lang="ts">
  import { AlertCircle, XCircle, HelpCircle, RefreshCw, ExternalLink } from 'lucide-svelte';
  import Button from './Button.svelte';
  import type { DiagnosedError } from '$lib/utils/errorDiagnosis';
  import { getCategoryLabel, getCategoryColor, isRetryable } from '$lib/utils/errorDiagnosis';

  interface Props {
    diagnosis: DiagnosedError;
    onRetry?: () => void;
    compact?: boolean;
  }

  const { diagnosis, onRetry, compact = false }: Props = $props();

  let showTechnicalDetails = $state(false);
  let showAllSuggestions = $state(false);

  const canRetry = $derived(isRetryable(diagnosis) && onRetry);
  const displaySuggestions = $derived(
    showAllSuggestions ? diagnosis.suggestions : diagnosis.suggestions.slice(0, 3)
  );
</script>

<div class="border border-red-200 dark:border-red-800 rounded-lg p-4 bg-red-50 dark:bg-red-900/20">
  <div class="flex items-start gap-3">
    <!-- Icon -->
    <div class="flex-shrink-0 mt-0.5">
      {#if diagnosis.category === 'unknown'}
        <HelpCircle size={20} class="text-red-600 dark:text-red-400" />
      {:else}
        <AlertCircle size={20} class="text-red-600 dark:text-red-400" />
      {/if}
    </div>

    <!-- Content -->
    <div class="flex-1 min-w-0">

      <!-- Header -->
      <div class="flex items-start justify-between gap-3 mb-2">
        <div class="flex-1">
          <h5 class="text-sm font-semibold text-red-800 dark:text-red-200 mb-1">
            {diagnosis.diagnosis}
          </h5>
          {#if !compact}
            <span class="inline-flex text-xs px-2 py-1 rounded-full {getCategoryColor(diagnosis.category)}">
              {getCategoryLabel(diagnosis.category)}
            </span>
          {/if}
        </div>

        <!-- Retry Button -->
        {#if canRetry}
          <Button
            variant="ghost"
            size="sm"
            onclick={onRetry}
            class="flex-shrink-0"
          >
            <RefreshCw size={14} />
            Retry
          </Button>
        {/if}
      </div>

      <!-- Suggestions -->
      {#if diagnosis.suggestions.length > 0}
        <div class="text-xs text-red-700 dark:text-red-300 mb-3">
          <strong class="font-medium">How to fix:</strong>
          <ul class="list-disc list-inside space-y-1 mt-1.5 ml-1">
            {#each displaySuggestions as suggestion}
              <li class="leading-relaxed">{suggestion}</li>
            {/each}
          </ul>

          <!-- Show more/less toggle -->
          {#if diagnosis.suggestions.length > 3}
            <button
              onclick={() => showAllSuggestions = !showAllSuggestions}
              class="text-red-600 dark:text-red-400 hover:underline mt-2 inline-flex items-center gap-1"
            >
              {showAllSuggestions ? 'Show less' : `Show ${diagnosis.suggestions.length - 3} more suggestions`}
              <svg
                class="w-3 h-3 transition-transform {showAllSuggestions ? 'rotate-180' : ''}"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
              </svg>
            </button>
          {/if}
        </div>
      {/if}

      <!-- Technical Details -->
      <details bind:open={showTechnicalDetails} class="text-xs group">
        <summary class="cursor-pointer text-red-600 dark:text-red-400 hover:underline flex items-center gap-1.5 select-none">
          <svg
            class="w-3 h-3 transition-transform group-open:rotate-90"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
          Show technical details
        </summary>

        <div class="mt-2 space-y-2">
          <!-- Original Error -->
          <div>
            <div class="font-medium text-red-700 dark:text-red-300 mb-1">Original Error:</div>
            <pre class="bg-red-100 dark:bg-red-900/40 p-2 rounded overflow-x-auto text-red-900 dark:text-red-100 whitespace-pre-wrap">{diagnosis.originalError}</pre>
          </div>

          <!-- Technical Context -->
          {#if diagnosis.technicalDetails && Object.keys(diagnosis.technicalDetails).length > 0}
            <div>
              <div class="font-medium text-red-700 dark:text-red-300 mb-1">Context:</div>
              <div class="bg-red-100 dark:bg-red-900/40 p-2 rounded text-red-900 dark:text-red-100">
                {#each Object.entries(diagnosis.technicalDetails) as [key, value]}
                  <div class="flex items-center gap-2">
                    <span class="font-medium">{key}:</span>
                    <span class="font-mono">{JSON.stringify(value)}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </details>

      <!-- Documentation Link -->
      {#if diagnosis.documentation}
        <a
          href={diagnosis.documentation}
          target="_blank"
          rel="noopener noreferrer"
          class="text-xs text-blue-600 dark:text-blue-400 hover:underline mt-2 inline-flex items-center gap-1"
        >
          View documentation
          <ExternalLink size={12} />
        </a>
      {/if}
    </div>
  </div>
</div>
