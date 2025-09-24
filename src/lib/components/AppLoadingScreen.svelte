<!--
  MCP Studio Application Loading Screen
  Displays initialization progress with enterprise-grade loading UI
-->
<script lang="ts">
  import {
    appStoreIsReady,
    appStoreLoadingSteps,
    appStoreCompletedSteps,
    appStoreTotalSteps,
    appStoreState
  } from '$lib/stores/appStore';

  // Reactive state using Svelte 5 runes - access store values directly
  let isReady = $state(false);
  let loadingSteps = $state([]);
  let completedSteps = $state(0);
  let totalSteps = $state(4);
  let appState = $state({
    isInitializing: true,
    databaseReady: false,
    mcpManagerReady: false,
    initializationError: null,
    startupTime: null
  });

  // Subscribe to store changes and update reactive state
  $effect(() => {
    const unsubscribeReady = appStoreIsReady.subscribe(value => {
      isReady = value;
    });
    const unsubscribeSteps = appStoreLoadingSteps.subscribe(value => {
      loadingSteps = value;
    });
    const unsubscribeCompleted = appStoreCompletedSteps.subscribe(value => {
      completedSteps = value;
    });
    const unsubscribeTotal = appStoreTotalSteps.subscribe(value => {
      totalSteps = value;
    });
    const unsubscribeState = appStoreState.subscribe(value => {
      appState = value;
    });

    return () => {
      unsubscribeReady();
      unsubscribeSteps();
      unsubscribeCompleted();
      unsubscribeTotal();
      unsubscribeState();
    };
  });

  // Progress calculation
  const progressPercentage = $derived(Math.round((completedSteps / totalSteps) * 100));

  // Show loading screen until app is ready
  const shouldShow = $derived(!isReady);
</script>

{#if shouldShow}
  <div class="mcp-loading-overlay" role="status" aria-live="polite">
    <div class="mcp-loading-container">
      <!-- Brand Header -->
      <div class="mcp-loading-header">
        <div class="mcp-loading-logo">
          <div class="mcp-logo-icon">🔌</div>
          <h1 class="mcp-logo-text">MCP Studio</h1>
        </div>
        <p class="mcp-logo-subtitle">Professional MCP Development Environment</p>
      </div>

      <!-- Progress Section -->
      <div class="mcp-loading-progress">
        <!-- Progress Bar -->
        <div class="mcp-progress-container">
          <div class="mcp-progress-bar">
            <div
              class="mcp-progress-fill"
              style="width: {progressPercentage}%"
              role="progressbar"
              aria-valuenow={progressPercentage}
              aria-valuemin="0"
              aria-valuemax="100"
            ></div>
          </div>
          <div class="mcp-progress-text">
            {progressPercentage}% Complete
          </div>
        </div>

        <!-- Loading Steps -->
        <div class="mcp-loading-steps">
          {#each loadingSteps as step (step.id)}
            <div class="mcp-loading-step" class:completed={step.status === 'completed'} class:loading={step.status === 'loading'} class:error={step.status === 'error'}>
              <div class="mcp-step-indicator">
                {#if step.status === 'completed'}
                  <div class="mcp-step-icon mcp-step-check">✓</div>
                {:else if step.status === 'loading'}
                  <div class="mcp-step-icon mcp-step-spinner">⟳</div>
                {:else if step.status === 'error'}
                  <div class="mcp-step-icon mcp-step-error">⚠</div>
                {:else}
                  <div class="mcp-step-icon mcp-step-pending">○</div>
                {/if}
              </div>
              <div class="mcp-step-content">
                <div class="mcp-step-label">{step.label}</div>
                {#if step.error}
                  <div class="mcp-step-error-text">{step.error}</div>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        <!-- Error State -->
        {#if appState.initializationError}
          <div class="mcp-loading-error">
            <div class="mcp-error-icon">⚠</div>
            <div class="mcp-error-content">
              <h3 class="mcp-error-title">Initialization Failed</h3>
              <p class="mcp-error-message">{appState.initializationError}</p>
              <button
                class="btn btn-primary btn-sm"
                onclick={() => window.location.reload()}
              >
                Retry
              </button>
            </div>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="mcp-loading-footer">
        <p class="mcp-loading-tip">
          {#if progressPercentage < 50}
            Setting up your MCP development environment...
          {:else if progressPercentage < 80}
            Loading server configurations...
          {:else}
            Almost ready! Finalizing initialization...
          {/if}
        </p>
      </div>
    </div>
  </div>
{/if}

<style>
  .mcp-loading-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--mcp-surface-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    min-height: 100vh;
  }

  .mcp-loading-container {
    max-width: 480px;
    width: 100%;
    padding: var(--mcp-space-8);
    text-align: center;
  }

  /* Brand Header */
  .mcp-loading-header {
    margin-bottom: var(--mcp-space-8);
  }

  .mcp-loading-logo {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--mcp-space-3);
    margin-bottom: var(--mcp-space-4);
  }

  .mcp-logo-icon {
    font-size: 3rem;
    opacity: 0.9;
  }

  .mcp-logo-text {
    font-size: var(--mcp-text-4xl);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-text-primary);
    margin: 0;
  }

  .mcp-logo-subtitle {
    font-size: var(--mcp-text-lg);
    color: var(--mcp-text-secondary);
    margin: 0;
    font-weight: var(--mcp-font-medium);
  }

  /* Progress Section */
  .mcp-loading-progress {
    margin-bottom: var(--mcp-space-8);
  }

  .mcp-progress-container {
    margin-bottom: var(--mcp-space-6);
  }

  .mcp-progress-bar {
    width: 100%;
    height: 8px;
    background: var(--mcp-surface-secondary);
    border-radius: var(--mcp-radius-full);
    overflow: hidden;
    margin-bottom: var(--mcp-space-2);
  }

  .mcp-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--mcp-primary-500), var(--mcp-primary-600));
    border-radius: var(--mcp-radius-full);
    transition: width var(--mcp-transition-base);
  }

  .mcp-progress-text {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-secondary);
  }

  /* Loading Steps */
  .mcp-loading-steps {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-3);
    text-align: left;
  }

  .mcp-loading-step {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3);
    border-radius: var(--mcp-radius-md);
    transition: all var(--mcp-transition-fast);
  }

  .mcp-loading-step.completed {
    background: var(--mcp-success-50);
    border: 1px solid var(--mcp-success-200);
  }

  .mcp-loading-step.loading {
    background: var(--mcp-primary-50);
    border: 1px solid var(--mcp-primary-200);
  }

  .mcp-loading-step.error {
    background: var(--mcp-error-50);
    border: 1px solid var(--mcp-error-200);
  }

  .mcp-step-indicator {
    flex-shrink: 0;
  }

  .mcp-step-icon {
    width: 24px;
    height: 24px;
    border-radius: var(--mcp-radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-bold);
  }

  .mcp-step-check {
    background: var(--mcp-success-500);
    color: white;
  }

  .mcp-step-spinner {
    background: var(--mcp-primary-500);
    color: white;
    animation: spin 1s linear infinite;
  }

  .mcp-step-error {
    background: var(--mcp-error-500);
    color: white;
  }

  .mcp-step-pending {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-tertiary);
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .mcp-step-content {
    flex: 1;
  }

  .mcp-step-label {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-primary);
  }

  .mcp-step-error-text {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-error-600);
    margin-top: var(--mcp-space-1);
  }

  /* Error State */
  .mcp-loading-error {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-4);
    background: var(--mcp-error-50);
    border: 1px solid var(--mcp-error-200);
    border-radius: var(--mcp-radius-lg);
    margin-top: var(--mcp-space-4);
    text-align: left;
  }

  .mcp-error-icon {
    font-size: var(--mcp-text-2xl);
    color: var(--mcp-error-500);
    flex-shrink: 0;
  }

  .mcp-error-content {
    flex: 1;
  }

  .mcp-error-title {
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-error-700);
    margin: 0 0 var(--mcp-space-1) 0;
  }

  .mcp-error-message {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-error-600);
    margin: 0 0 var(--mcp-space-3) 0;
  }

  /* Footer */
  .mcp-loading-footer {
    border-top: 1px solid var(--mcp-border-primary);
    padding-top: var(--mcp-space-6);
  }

  .mcp-loading-tip {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-tertiary);
    margin: 0;
    font-style: italic;
  }

  /* Dark mode adjustments using :global() for proper scoping */
  :global([data-theme="dark"]) .mcp-loading-step.completed {
    background: rgba(34, 197, 94, 0.1);
    border-color: rgba(34, 197, 94, 0.3);
  }

  :global([data-theme="dark"]) .mcp-loading-step.loading {
    background: rgba(59, 130, 246, 0.1);
    border-color: rgba(59, 130, 246, 0.3);
  }

  :global([data-theme="dark"]) .mcp-loading-step.error {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.3);
  }

  :global([data-theme="dark"]) .mcp-loading-error {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.3);
  }

  /* Dark mode text improvements for better readability */
  :global([data-theme="dark"]) .mcp-step-label {
    color: var(--mcp-text-primary-dark, rgba(255, 255, 255, 0.9));
  }

  :global([data-theme="dark"]) .mcp-step-error-text {
    color: var(--mcp-error-400, rgba(248, 113, 113, 0.9));
  }

  :global([data-theme="dark"]) .mcp-progress-text {
    color: var(--mcp-text-secondary-dark, rgba(255, 255, 255, 0.7));
  }

  :global([data-theme="dark"]) .mcp-logo-text {
    color: var(--mcp-text-primary-dark, rgba(255, 255, 255, 0.95));
  }

  :global([data-theme="dark"]) .mcp-logo-subtitle {
    color: var(--mcp-text-secondary-dark, rgba(255, 255, 255, 0.7));
  }

  /* Mobile responsiveness */
  @media (max-width: 767px) {
    .mcp-loading-container {
      padding: var(--mcp-space-6);
    }

    .mcp-logo-text {
      font-size: var(--mcp-text-3xl);
    }

    .mcp-logo-subtitle {
      font-size: var(--mcp-text-base);
    }

    .mcp-loading-steps {
      gap: var(--mcp-space-2);
    }

    .mcp-loading-step {
      padding: var(--mcp-space-2);
    }
  }

  /* Reduced motion support */
  @media (prefers-reduced-motion: reduce) {
    .mcp-step-spinner {
      animation: none;
    }

    .mcp-progress-fill {
      transition: none;
    }
  }
</style>