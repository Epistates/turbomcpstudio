<!--
  SamplingRequestCard - Display sampling request in queue lists

  Features:
  - Pending vs completed visual states
  - Tool context display (compact mode)
  - Message preview for pending requests
  - Metrics display (duration, cost, message count)
  - Click to select request
  - Dark mode support
-->
<script lang="ts">
  import { Workflow, CheckCircle, Clock } from 'lucide-svelte';
  import type { SamplingRequest } from '$lib/types/sampling';
  import { formatMessageContent } from '$lib/utils/samplingUtils';
  import ToolContextBadge from './ToolContextBadge.svelte';

  const { request, selected, onclick } = $props<{
    request: SamplingRequest;
    selected: boolean;
    onclick: () => void;
  }>();

  const isPending = $derived(request.status === 'pending');

  // Visual styling based on status
  const bgClass = $derived(
    isPending
      ? 'bg-orange-50 hover:bg-orange-100 border-orange-200'
      : 'bg-gray-50 hover:bg-gray-100 border-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 dark:border-gray-600'
  );

  const ringClass = $derived(
    selected
      ? isPending
        ? 'ring-2 ring-orange-400'
        : 'ring-2 ring-mcp-primary-400'
      : ''
  );

  const iconColor = $derived(isPending ? 'text-orange-600' : 'text-green-600');
  const statusText = $derived(isPending ? 'Pending' : request.status);
  const statusBadgeClass = $derived(
    isPending
      ? 'text-xs text-orange-700 bg-orange-100 px-2 py-1 rounded-full'
      : 'text-xs px-2 py-1 rounded-full border'
  );

  // Get status color for completed requests
  function getStatusColor(status: string): string {
    const colors = {
      completed: 'text-green-700 bg-green-100 border-green-200',
      error: 'text-red-700 bg-red-100 border-red-200',
      rejected: 'text-red-700 bg-red-100 border-red-200'
    };
    return colors[status as keyof typeof colors] || 'text-gray-700 bg-gray-100 border-gray-200';
  }

  const userMessage = $derived(request.messages.find((m: any) => m.role === 'user'));
</script>

<button
  {onclick}
  class="w-full p-3 text-left border rounded-lg transition-colors {bgClass} {ringClass}"
>
  <!-- Header: Server name and status -->
  <div class="flex items-center justify-between mb-2">
    <div class="flex items-center gap-2">
      {#if isPending}
        <Workflow size={12} class={iconColor} />
      {:else}
        <CheckCircle size={12} class={iconColor} />
      {/if}
      <span class="text-sm font-medium text-gray-900 dark:text-white">
        {request.serverName}
      </span>
    </div>
    <span class={isPending ? statusBadgeClass : `${statusBadgeClass} ${getStatusColor(request.status)}`}>
      {statusText}
    </span>
  </div>

  <!-- Tool Context (if present) -->
  {#if request.toolContext}
    <div class="mb-2">
      {#if isPending}
        <!-- Pending requests use special styling -->
        <div class="p-2 bg-white/50 border border-orange-200 rounded">
          <ToolContextBadge
            toolContext={request.toolContext}
            operationContext={request.operationContext}
            variant="compact"
          />
        </div>
      {:else}
        <ToolContextBadge
          toolContext={request.toolContext}
          operationContext={request.operationContext}
          variant="compact"
        />
      {/if}
    </div>
  {/if}

  <!-- Message preview (pending only) or cost info (completed) -->
  {#if isPending && userMessage}
    <p class="text-xs text-gray-600 line-clamp-2 mb-2 dark:text-gray-400">
      {formatMessageContent(userMessage.content)}
    </p>
  {/if}

  <!-- Footer: Metadata -->
  <div class="flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
    <span>
      {#if !isPending && request.duration}
        {request.duration}ms
        {#if request.cost}
          • ${request.cost.toFixed(4)}
        {/if}
      {:else}
        {request.messages.length} message{request.messages.length !== 1 ? 's' : ''}
      {/if}
    </span>
    <span>{new Date(request.timestamp).toLocaleTimeString()}</span>
  </div>
</button>

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
