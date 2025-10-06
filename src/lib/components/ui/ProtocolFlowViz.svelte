<script lang="ts">
  import { ArrowRight, ArrowLeft, Activity, Copy, ChevronDown, ChevronRight } from 'lucide-svelte';
  import Button from './Button.svelte';

  interface ProtocolMessage {
    direction: 'outgoing' | 'incoming';
    method?: string;
    type: string; // 'request' | 'response' | 'processing'
    timestamp: string;
    payload: any;
    latency?: number;
  }

  interface Props {
    messages: ProtocolMessage[];
    compact?: boolean;
  }

  const { messages, compact = false }: Props = $props();

  let expandedMessages = $state<Set<number>>(new Set());

  function toggleMessage(index: number) {
    if (expandedMessages.has(index)) {
      expandedMessages.delete(index);
    } else {
      expandedMessages.add(index);
    }
    expandedMessages = new Set(expandedMessages);
  }

  function copyPayload(payload: any) {
    navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
  }

  function getMessageColor(message: ProtocolMessage): string {
    if (message.type === 'processing') return 'border-yellow-200 dark:border-yellow-800 bg-yellow-50 dark:bg-yellow-900/20';
    if (message.direction === 'outgoing') return 'border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/20';
    return 'border-green-200 dark:border-green-800 bg-green-50 dark:bg-green-900/20';
  }

  function getArrowColor(message: ProtocolMessage): string {
    if (message.type === 'processing') return 'text-yellow-600 dark:text-yellow-400';
    if (message.direction === 'outgoing') return 'text-blue-600 dark:text-blue-400';
    return 'text-green-600 dark:text-green-400';
  }
</script>

<div class="space-y-3">
  {#each messages as message, index}
    <div class="border rounded-lg {getMessageColor(message)} transition-all">
      <!-- Message Header -->
      <button
        onclick={() => toggleMessage(index)}
        class="w-full px-4 py-3 flex items-center justify-between text-left hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
      >
        <div class="flex items-center gap-3 flex-1">
          <!-- Direction Arrow -->
          {#if message.type === 'processing'}
            <Activity size={16} class="animate-pulse {getArrowColor(message)} flex-shrink-0" />
          {:else if message.direction === 'outgoing'}
            <ArrowRight size={16} class="{getArrowColor(message)} flex-shrink-0" />
          {:else}
            <ArrowLeft size={16} class="{getArrowColor(message)} flex-shrink-0" />
          {/if}

          <!-- Message Type -->
          <div class="flex-1">
            <div class="text-sm font-medium text-gray-900 dark:text-white">
              {#if message.direction === 'outgoing'}
                Client → Server
              {:else if message.direction === 'incoming'}
                Server → Client
              {:else}
                Server Processing
              {/if}
            </div>
            <div class="text-xs text-gray-600 dark:text-gray-400 mt-0.5">
              {#if message.method}
                <span class="font-mono">{message.method}</span>
              {/if}
              {#if message.type === 'response'}
                <span class="font-mono">Response</span>
              {/if}
              {#if message.latency}
                <span class="ml-2">• {message.latency}ms</span>
              {/if}
            </div>
          </div>

          <!-- Timestamp -->
          {#if !compact}
            <div class="text-xs text-gray-500 dark:text-gray-400">
              {new Date(message.timestamp).toLocaleTimeString()}
            </div>
          {/if}

          <!-- Expand/Collapse Icon -->
          <div class="flex-shrink-0">
            {#if expandedMessages.has(index)}
              <ChevronDown size={16} class="text-gray-400" />
            {:else}
              <ChevronRight size={16} class="text-gray-400" />
            {/if}
          </div>
        </div>
      </button>

      <!-- Expanded Payload -->
      {#if expandedMessages.has(index)}
        <div class="border-t border-gray-200 dark:border-gray-700 p-4">
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs font-medium text-gray-700 dark:text-gray-300">
              {message.type === 'response' ? 'Response Payload' : 'Request Payload'}
            </span>
            <Button
              variant="ghost"
              size="sm"
              onclick={() => copyPayload(message.payload)}
            >
              <Copy size={12} />
              Copy
            </Button>
          </div>

          <div class="bg-white dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-600 p-3 overflow-x-auto">
            <pre class="text-xs text-gray-800 dark:text-gray-200 font-mono">{JSON.stringify(message.payload, null, 2)}</pre>
          </div>
        </div>
      {/if}
    </div>
  {/each}

  {#if messages.length === 0}
    <div class="text-center py-8 text-gray-500 dark:text-gray-400 text-sm">
      No protocol messages yet. Send a sampling request to see the MCP protocol flow.
    </div>
  {/if}
</div>
