<!--
  Empty State Component for Missing MCP Capabilities
  Shows when no servers support a specific MCP capability
-->
<script lang="ts">
  import { Database, Plug, AlertCircle } from 'lucide-svelte';
  import Button from './Button.svelte';
  import { uiStore } from '$lib/stores/uiStore';
  import type { McpCapability } from '$lib/utils/serverCapabilities';

  // Props using Svelte 5 runes
  const {
    capability,
    totalServers = 0,
    connectedServers = 0
  }: {
    capability: McpCapability;
    totalServers?: number;
    connectedServers?: number;
  } = $props();

  // Capability-specific messaging
  const capabilityConfig = {
    tools: {
      icon: '🔧',
      title: 'No Tool Servers Available',
      description: 'None of your connected servers support tool execution.',
      suggestion: 'Connect to a server that implements MCP tools to execute commands and functions.'
    },
    resources: {
      icon: '📁',
      title: 'No Resource Servers Available',
      description: 'None of your connected servers provide accessible resources.',
      suggestion: 'Connect to a server that implements MCP resources to browse files and data.'
    },
    prompts: {
      icon: '💭',
      title: 'No Prompt Servers Available',
      description: 'None of your connected servers provide prompt templates.',
      suggestion: 'Connect to a server that implements MCP prompts to use pre-built templates.'
    },
    sampling: {
      icon: '🎯',
      title: 'No Sampling Servers Available',
      description: 'None of your connected servers support LLM sampling.',
      suggestion: 'Connect to a server that implements MCP sampling to test AI model interactions.'
    },
    elicitation: {
      icon: '💬',
      title: 'No Elicitation Servers Available',
      description: 'None of your connected servers support user elicitation.',
      suggestion: 'Connect to a server that implements MCP elicitation to collect user input.'
    }
  };

  const config = capabilityConfig[capability];

  function addServer() {
    uiStore.openModal('addServer');
  }

  // Determine the appropriate message based on server state
  const message = $derived(
    totalServers === 0 ? {
      title: 'No Servers Configured',
      description: 'Get started by adding your first MCP server.',
      showAddButton: true
    } : connectedServers === 0 ? {
      title: 'No Servers Connected',
      description: `You have ${totalServers} server${totalServers === 1 ? '' : 's'} configured but none are currently connected.`,
      showAddButton: false
    } : {
      title: config.title,
      description: config.description,
      showAddButton: true
    }
  );
</script>

<div class="empty-capability-state">
  <div class="empty-capability-state__content">
    <!-- Icon -->
    <div class="empty-capability-state__icon">
      <Database size={48} class="empty-capability-state__icon-bg" />
      <div class="empty-capability-state__icon-overlay">
        {config.icon}
      </div>
    </div>

    <!-- Content -->
    <div class="empty-capability-state__text">
      <h3 class="empty-capability-state__title">
        {message.title}
      </h3>
      <p class="empty-capability-state__description">
        {message.description}
      </p>

      {#if connectedServers > 0 && config.suggestion}
        <p class="empty-capability-state__suggestion">
          <AlertCircle size={16} />
          {config.suggestion}
        </p>
      {/if}
    </div>

    <!-- Actions -->
    <div class="empty-capability-state__actions">
      {#if message.showAddButton}
        <Button
          variant="primary"
          onclick={addServer}
        >
          <Plug size={16} />
          Add MCP Server
        </Button>
      {/if}

      {#if connectedServers === 0 && totalServers > 0}
        <p class="empty-capability-state__hint">
          Check your server connections in the sidebar
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  .empty-capability-state {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 400px;
    padding: var(--mcp-space-8);
    text-align: center;
  }

  .empty-capability-state__content {
    max-width: 400px;
    width: 100%;
  }

  .empty-capability-state__icon {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-bottom: var(--mcp-space-6);
  }

  .empty-capability-state__icon-bg {
    color: var(--mcp-text-tertiary);
    opacity: 0.3;
  }

  .empty-capability-state__icon-overlay {
    position: absolute;
    font-size: 24px;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
  }

  .empty-capability-state__text {
    margin-bottom: var(--mcp-space-6);
  }

  .empty-capability-state__title {
    font-size: var(--mcp-text-xl);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0 0 var(--mcp-space-3) 0;
  }

  .empty-capability-state__description {
    font-size: var(--mcp-text-base);
    color: var(--mcp-text-secondary);
    margin: 0 0 var(--mcp-space-4) 0;
    line-height: var(--mcp-leading-relaxed);
  }

  .empty-capability-state__suggestion {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--mcp-space-2);
    font-size: var(--mcp-text-sm);
    color: var(--mcp-warning-700);
    background: var(--mcp-warning-50);
    border: 1px solid var(--mcp-warning-200);
    border-radius: var(--mcp-radius-md);
    padding: var(--mcp-space-3);
    margin: 0;
    line-height: var(--mcp-leading-normal);
  }

  .empty-capability-state__actions {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--mcp-space-3);
  }

  .empty-capability-state__hint {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-tertiary);
    margin: 0;
  }

  /* Dark theme adjustments */
  [data-theme="dark"] .empty-capability-state__suggestion {
    color: var(--mcp-warning-200);
    background: var(--mcp-warning-900);
    border-color: var(--mcp-warning-700);
  }
</style>