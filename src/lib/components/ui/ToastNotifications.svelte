<script lang="ts">
  import { uiStore } from '$lib/stores/uiStore';
  import { TIMEOUTS } from '$lib/constants';
  import { CheckCircle, AlertCircle, Info, X, Copy, Check } from 'lucide-svelte';
  import { onMount } from 'svelte';

  // Subscribe to UI store
  const ui = $derived($uiStore);
  const notification = $derived(ui.notification);

  // Copy state
  let copied = $state(false);

  async function copyToClipboard() {
    if (!notification) return;

    try {
      const text = `${notification.message}`;
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => copied = false, TIMEOUTS.TOAST_DURATION);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }

  function closeNotification() {
    uiStore.clearNotification();
  }

  function getIcon(type: string) {
    switch (type) {
      case 'success': return CheckCircle;
      case 'error': return AlertCircle;
      case 'warning': return AlertCircle;
      default: return Info;
    }
  }
</script>

{#if notification}
  {@const IconComponent = getIcon(notification.type)}
  <div class="toast-container">
    <div class="toast toast--{notification.type}" role="alert">
      <div class="toast__icon">
        <IconComponent size={20} />
      </div>

      <div class="toast__content">
        <p class="toast__message">{notification.message}</p>
      </div>

      <div class="toast__actions">
        <button
          class="toast__action"
          onclick={copyToClipboard}
          title="Copy to clipboard"
        >
          {#if copied}
            <Check size={16} />
          {:else}
            <Copy size={16} />
          {/if}
        </button>

        <button
          class="toast__action"
          onclick={closeNotification}
          title="Dismiss"
        >
          <X size={16} />
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    top: 80px;
    right: var(--mcp-space-4);
    z-index: 1000;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: var(--mcp-space-3);
    min-width: 320px;
    max-width: 420px;
    padding: var(--mcp-space-4);
    background: var(--mcp-surface-secondary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-lg);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    pointer-events: auto;
    animation: slideIn 0.3s ease-out;
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  .toast__icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .toast--success .toast__icon {
    color: var(--mcp-success-500);
  }

  .toast--error .toast__icon {
    color: var(--mcp-error-500);
  }

  .toast--warning .toast__icon {
    color: var(--mcp-warning-500);
  }

  .toast--info .toast__icon {
    color: var(--mcp-primary-500);
  }

  .toast__content {
    flex: 1;
    min-width: 0;
  }

  .toast__message {
    margin: 0;
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-primary);
    line-height: var(--mcp-leading-relaxed);
    word-wrap: break-word;
  }

  .toast__actions {
    display: flex;
    gap: var(--mcp-space-1);
    flex-shrink: 0;
  }

  .toast__action {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
  }

  .toast__action:hover {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-primary);
  }

  .toast__action:active {
    transform: scale(0.95);
  }

  /* Dark theme */
  [data-theme="dark"] .toast {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  /* Responsive */
  @media (max-width: 767px) {
    .toast-container {
      right: var(--mcp-space-2);
      left: var(--mcp-space-2);
    }

    .toast {
      min-width: auto;
      width: 100%;
    }
  }

  /* Reduced motion */
  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }
</style>
