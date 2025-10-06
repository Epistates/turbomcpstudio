<!--
  Real-time Workflow Execution Monitor
  Displays live progress, step results, and variable state during workflow execution
-->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import type { WorkflowExecution, WorkflowExecutionEvent } from '$lib/types/collections';
  import {
    Play,
    Pause,
    Square,
    CheckCircle,
    AlertCircle,
    Clock,
    Zap,
    Database,
    FileText,
    MessageSquare,
    Activity,
    ChevronRight,
    ChevronDown
  } from 'lucide-svelte';

  // Props using Svelte 5 runes
  const { executionId = '' } = $props();

  // State
  let execution: WorkflowExecution | null = $state(null);
  let events: WorkflowExecutionEvent[] = $state([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let unlistenFn: UnlistenFn | null = null;
  let expandedSteps = $state(new Set<string>());

  // Lifecycle
  onMount(async () => {
    if (!executionId) return;

    try {
      // Load initial execution state
      execution = await invoke('get_workflow_execution', { executionId: executionId });

      if (execution) {
        events = execution.events || [];
      }

      // Listen for real-time workflow events
      unlistenFn = await listen('workflow-event', (event) => {
        const workflowEvent = event.payload as WorkflowExecutionEvent;

        // Only process events for our execution
        if (workflowEvent.execution_id === executionId) {
          events = [...events, workflowEvent];

          // Update execution status based on event
          if (execution) {
            if (workflowEvent.event_type === 'execution_completed') {
              execution.status = 'completed';
            } else if (workflowEvent.event_type === 'execution_failed') {
              execution.status = 'failed';
            }
            execution.events = events;
          }
        }
      });

      isLoading = false;
    } catch (err) {
      error = `Failed to load execution: ${err}`;
      isLoading = false;
    }
  });

  onDestroy(() => {
    if (unlistenFn) {
      unlistenFn();
    }
  });

  function getOperationIcon(operationType: string) {
    switch (operationType.toLowerCase()) {
      case 'tool': return Zap;
      case 'resource': return Database;
      case 'prompt': return FileText;
      case 'sampling': return MessageSquare;
      case 'elicitation': return Activity;
      default: return CheckCircle;
    }
  }

  function getStatusIcon(status: string) {
    switch (status?.toLowerCase()) {
      case 'completed': return CheckCircle;
      case 'failed': return AlertCircle;
      case 'running': return Clock;
      default: return Clock;
    }
  }

  function getStatusClass(status: string) {
    switch (status?.toLowerCase()) {
      case 'completed': return 'status-success';
      case 'failed': return 'status-error';
      case 'running': return 'status-running';
      default: return 'status-pending';
    }
  }

  function formatDuration(startTime?: string, endTime?: string) {
    if (!startTime) return '-';
    const start = new Date(startTime).getTime();
    const end = endTime ? new Date(endTime).getTime() : Date.now();
    const duration = end - start;

    if (duration < 1000) return `${duration}ms`;
    if (duration < 60000) return `${(duration / 1000).toFixed(1)}s`;
    return `${Math.floor(duration / 60000)}m ${Math.floor((duration % 60000) / 1000)}s`;
  }

  function toggleStepExpansion(stepId: string) {
    if (expandedSteps.has(stepId)) {
      expandedSteps.delete(stepId);
    } else {
      expandedSteps.add(stepId);
    }
    expandedSteps = new Set(expandedSteps);
  }

  async function stopExecution() {
    if (!executionId) return;

    try {
      await invoke('stop_workflow_execution', { executionId: executionId });
    } catch (err) {
      error = `Failed to stop execution: ${err}`;
    }
  }
</script>

<div class="execution-monitor">
  {#if isLoading}
    <div class="execution-monitor__loading">
      <Clock size={24} class="execution-monitor__loading-icon" />
      <span>Loading execution details...</span>
    </div>
  {:else if error}
    <div class="execution-monitor__error">
      <AlertCircle size={24} class="execution-monitor__error-icon" />
      <span>{error}</span>
    </div>
  {:else if execution}
    <!-- Execution Header -->
    <div class="execution-monitor__header">
      <div class="execution-monitor__header-main">
        <div class="execution-monitor__status">
          {#if execution}
            {@const StatusIcon = getStatusIcon(execution.status)}
            <StatusIcon size={20} class="execution-monitor__status-icon {getStatusClass(execution.status)}" />
          {/if}
          <h2 class="execution-monitor__title">Workflow Execution</h2>
        </div>

        <div class="execution-monitor__controls">
          {#if execution.status === 'running'}
            <button
              class="execution-monitor__control-button execution-monitor__control-button--danger"
              onclick={stopExecution}
              title="Stop Execution"
            >
              <Square size={16} />
              Stop
            </button>
          {/if}
        </div>
      </div>

      <div class="execution-monitor__meta">
        <div class="execution-monitor__meta-item">
          <span class="execution-monitor__meta-label">Collection:</span>
          <span class="execution-monitor__meta-value">{execution.collection_name}</span>
        </div>
        <div class="execution-monitor__meta-item">
          <span class="execution-monitor__meta-label">Started:</span>
          <span class="execution-monitor__meta-value">
            {new Date(execution.started_at).toLocaleString()}
          </span>
        </div>
        {#if execution.finished_at}
          <div class="execution-monitor__meta-item">
            <span class="execution-monitor__meta-label">Duration:</span>
            <span class="execution-monitor__meta-value">
              {formatDuration(execution.started_at, execution.finished_at)}
            </span>
          </div>
        {:else if execution.status === 'running'}
          <div class="execution-monitor__meta-item">
            <span class="execution-monitor__meta-label">Running:</span>
            <span class="execution-monitor__meta-value">
              {formatDuration(execution.started_at)}
            </span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Variable State -->
    {#if execution.final_variables && Object.keys(execution.final_variables).length > 0}
      <div class="execution-monitor__section">
        <h3 class="execution-monitor__section-title">Variables</h3>
        <div class="execution-monitor__variables">
          {#each Object.entries(execution.final_variables) as [key, value]}
            <div class="execution-monitor__variable">
              <span class="execution-monitor__variable-key">${key}</span>
              <span class="execution-monitor__variable-value">
                {typeof value === 'string' ? value : JSON.stringify(value)}
              </span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Step Results -->
    <div class="execution-monitor__section">
      <h3 class="execution-monitor__section-title">
        Step Results ({execution.step_results ? Object.keys(execution.step_results).length : 0})
      </h3>

      {#if execution.step_results && Object.keys(execution.step_results).length > 0}
        <div class="execution-monitor__steps">
          {#each Object.values(execution.step_results) as stepResult}
            <div class="execution-monitor__step">
              <button
                class="execution-monitor__step-header"
                onclick={() => toggleStepExpansion(stepResult.step_id)}
              >
                <div class="execution-monitor__step-header-left">
                  {#if expandedSteps.has(stepResult.step_id)}
                    <ChevronDown size={16} />
                  {:else}
                    <ChevronRight size={16} />
                  {/if}

                  {#if stepResult.operation_type}
                    {@const OperationIcon = getOperationIcon(stepResult.operation_type)}
                    <OperationIcon size={16} class="execution-monitor__step-icon" />
                  {/if}

                  <span class="execution-monitor__step-name">{stepResult.step_name}</span>
                </div>

                <div class="execution-monitor__step-header-right">
                  {#if stepResult.status}
                    {@const StatusIcon = getStatusIcon(stepResult.status)}
                    <StatusIcon size={14} class="execution-monitor__step-status-icon {getStatusClass(stepResult.status)}" />
                  {/if}
                  <span class="execution-monitor__step-duration">
                    {formatDuration(stepResult.started_at, stepResult.finished_at)}
                  </span>
                </div>
              </button>

              {#if expandedSteps.has(stepResult.step_id)}
                <div class="execution-monitor__step-details">
                  <div class="execution-monitor__step-detail">
                    <h4 class="execution-monitor__step-detail-title">Operation</h4>
                    <div class="execution-monitor__step-detail-content">
                      <div class="execution-monitor__operation-info">
                        <span class="execution-monitor__operation-type">{stepResult.operation_type?.toUpperCase() || 'UNKNOWN'}</span>
                        <span class="execution-monitor__operation-target">{stepResult.operation_target}</span>
                      </div>
                    </div>
                  </div>

                  {#if stepResult.result}
                    <div class="execution-monitor__step-detail">
                      <h4 class="execution-monitor__step-detail-title">Result</h4>
                      <div class="execution-monitor__step-detail-content">
                        <pre class="execution-monitor__json-result">{JSON.stringify(stepResult.result, null, 2)}</pre>
                      </div>
                    </div>
                  {/if}

                  {#if stepResult.error}
                    <div class="execution-monitor__step-detail execution-monitor__step-detail--error">
                      <h4 class="execution-monitor__step-detail-title">Error</h4>
                      <div class="execution-monitor__step-detail-content">
                        <div class="execution-monitor__error-message">{stepResult.error}</div>
                      </div>
                    </div>
                  {/if}

                  {#if stepResult.extracted_variables && Object.keys(stepResult.extracted_variables).length > 0}
                    <div class="execution-monitor__step-detail">
                      <h4 class="execution-monitor__step-detail-title">Variables Extracted</h4>
                      <div class="execution-monitor__step-detail-content">
                        <div class="execution-monitor__extracted-variables">
                          {#each Object.entries(stepResult.extracted_variables) as [key, value]}
                            <div class="execution-monitor__extracted-variable">
                              <span class="execution-monitor__extracted-variable-key">${key}</span>
                              <span class="execution-monitor__extracted-variable-value">
                                {typeof value === 'string' ? value : JSON.stringify(value)}
                              </span>
                            </div>
                          {/each}
                        </div>
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <div class="execution-monitor__empty">
          <Activity size={32} class="execution-monitor__empty-icon" />
          <p class="execution-monitor__empty-text">No step results yet</p>
        </div>
      {/if}
    </div>

    <!-- Live Events -->
    {#if events.length > 0}
      <div class="execution-monitor__section">
        <h3 class="execution-monitor__section-title">Live Events ({events.length})</h3>
        <div class="execution-monitor__events">
          {#each events.slice().reverse() as event}
            <div class="execution-monitor__event execution-monitor__event--{event.event_type}">
              <div class="execution-monitor__event-timestamp">
                {new Date(event.timestamp).toLocaleTimeString()}
              </div>
              <div class="execution-monitor__event-content">
                <span class="execution-monitor__event-type">{event.event_type}</span>
                {#if event.step_id}
                  <span class="execution-monitor__event-step">Step: {event.step_id}</span>
                {/if}
                {#if event.message}
                  <span class="execution-monitor__event-message">{event.message}</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {:else}
    <div class="execution-monitor__empty">
      <Activity size={48} class="execution-monitor__empty-icon" />
      <p class="execution-monitor__empty-text">No execution found</p>
    </div>
  {/if}
</div>

<style>
  .execution-monitor {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--mcp-surface-primary);
    overflow: hidden;
  }

  /* Loading & Error States */
  .execution-monitor__loading,
  .execution-monitor__error,
  .execution-monitor__empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--mcp-space-8);
    gap: var(--mcp-space-3);
    color: var(--mcp-text-secondary);
  }

  .execution-monitor__loading-icon {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .execution-monitor__error-icon {
    color: var(--mcp-error-500);
  }

  .execution-monitor__empty-icon {
    color: var(--mcp-text-tertiary);
  }

  /* Header */
  .execution-monitor__header {
    padding: var(--mcp-space-6);
    border-bottom: 1px solid var(--mcp-border-primary);
    background: var(--mcp-surface-secondary);
  }

  .execution-monitor__header-main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--mcp-space-4);
  }

  .execution-monitor__status {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
  }

  .execution-monitor__status-icon.status-success {
    color: var(--mcp-success-500);
  }

  .execution-monitor__status-icon.status-error {
    color: var(--mcp-error-500);
  }

  .execution-monitor__status-icon.status-running {
    color: var(--mcp-primary-500);
    animation: pulse 2s ease-in-out infinite;
  }

  .execution-monitor__status-icon.status-pending {
    color: var(--mcp-text-tertiary);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .execution-monitor__title {
    font-size: var(--mcp-text-xl);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0;
  }

  .execution-monitor__controls {
    display: flex;
    gap: var(--mcp-space-2);
  }

  .execution-monitor__control-button {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    color: var(--mcp-text-secondary);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
  }

  .execution-monitor__control-button:hover {
    background: var(--mcp-surface-tertiary);
    border-color: var(--mcp-border-secondary);
  }

  .execution-monitor__control-button--danger {
    color: var(--mcp-error-600);
    border-color: var(--mcp-error-200);
  }

  .execution-monitor__control-button--danger:hover {
    background: var(--mcp-error-50);
    border-color: var(--mcp-error-300);
  }

  .execution-monitor__meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--mcp-space-6);
  }

  .execution-monitor__meta-item {
    display: flex;
    gap: var(--mcp-space-2);
  }

  .execution-monitor__meta-label {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-secondary);
  }

  .execution-monitor__meta-value {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-primary);
  }

  /* Sections */
  .execution-monitor__section {
    border-bottom: 1px solid var(--mcp-border-primary);
  }

  .execution-monitor__section:last-child {
    border-bottom: none;
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .execution-monitor__section-title {
    padding: var(--mcp-space-4) var(--mcp-space-6);
    margin: 0;
    font-size: var(--mcp-text-lg);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    background: var(--mcp-surface-secondary);
    border-bottom: 1px solid var(--mcp-border-primary);
  }

  /* Variables */
  .execution-monitor__variables {
    padding: var(--mcp-space-4) var(--mcp-space-6);
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--mcp-space-3);
  }

  .execution-monitor__variable {
    display: flex;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-secondary);
    border-radius: var(--mcp-radius-md);
  }

  .execution-monitor__variable-key {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-primary-600);
    white-space: nowrap;
  }

  .execution-monitor__variable-value {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
    word-break: break-all;
  }

  /* Steps */
  .execution-monitor__steps {
    flex: 1;
    overflow-y: auto;
    padding: var(--mcp-space-4) 0;
  }

  .execution-monitor__step {
    border-bottom: 1px solid var(--mcp-border-primary);
  }

  .execution-monitor__step:last-child {
    border-bottom: none;
  }

  .execution-monitor__step-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--mcp-space-4) var(--mcp-space-6);
    background: transparent;
    border: none;
    text-align: left;
    cursor: pointer;
    transition: background-color var(--mcp-transition-fast);
  }

  .execution-monitor__step-header:hover {
    background: var(--mcp-surface-secondary);
  }

  .execution-monitor__step-header-left {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    flex: 1;
    min-width: 0;
  }

  .execution-monitor__step-header-right {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
  }

  .execution-monitor__step-icon {
    color: var(--mcp-text-secondary);
    flex-shrink: 0;
  }

  .execution-monitor__step-name {
    font-size: var(--mcp-text-base);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .execution-monitor__step-status-icon {
    flex-shrink: 0;
  }

  .execution-monitor__step-duration {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-tertiary);
    white-space: nowrap;
  }

  /* Step Details */
  .execution-monitor__step-details {
    padding: 0 var(--mcp-space-6) var(--mcp-space-4);
    background: var(--mcp-surface-secondary);
  }

  .execution-monitor__step-detail {
    margin-bottom: var(--mcp-space-4);
  }

  .execution-monitor__step-detail:last-child {
    margin-bottom: 0;
  }

  .execution-monitor__step-detail--error {
    border-left: 4px solid var(--mcp-error-500);
    padding-left: var(--mcp-space-3);
  }

  .execution-monitor__step-detail-title {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
    margin: 0 0 var(--mcp-space-2) 0;
  }

  .execution-monitor__step-detail-content {
    background: var(--mcp-surface-primary);
    border: 1px solid var(--mcp-border-primary);
    border-radius: var(--mcp-radius-md);
    padding: var(--mcp-space-3);
  }

  .execution-monitor__operation-info {
    display: flex;
    gap: var(--mcp-space-3);
    align-items: center;
  }

  .execution-monitor__operation-type {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-primary-600);
    background: var(--mcp-primary-100);
    padding: 2px 6px;
    border-radius: var(--mcp-radius-sm);
  }

  .execution-monitor__operation-target {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
  }

  .execution-monitor__json-result {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 300px;
    overflow-y: auto;
  }

  .execution-monitor__error-message {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-sm);
    color: var(--mcp-error-600);
  }

  .execution-monitor__extracted-variables {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-2);
  }

  .execution-monitor__extracted-variable {
    display: flex;
    gap: var(--mcp-space-3);
  }

  .execution-monitor__extracted-variable-key {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-primary-600);
  }

  .execution-monitor__extracted-variable-value {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
  }

  /* Events */
  .execution-monitor__events {
    flex: 1;
    overflow-y: auto;
    padding: var(--mcp-space-4);
  }

  .execution-monitor__event {
    display: flex;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3);
    margin-bottom: var(--mcp-space-2);
    border-radius: var(--mcp-radius-md);
    border-left: 4px solid var(--mcp-border-secondary);
  }

  .execution-monitor__event--step_started {
    border-left-color: var(--mcp-primary-500);
    background: var(--mcp-primary-50);
  }

  .execution-monitor__event--step_completed {
    border-left-color: var(--mcp-success-500);
    background: var(--mcp-success-50);
  }

  .execution-monitor__event--step_failed {
    border-left-color: var(--mcp-error-500);
    background: var(--mcp-error-50);
  }

  .execution-monitor__event--execution_completed {
    border-left-color: var(--mcp-success-500);
    background: var(--mcp-success-50);
  }

  .execution-monitor__event--execution_failed {
    border-left-color: var(--mcp-error-500);
    background: var(--mcp-error-50);
  }

  .execution-monitor__event-timestamp {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    white-space: nowrap;
  }

  .execution-monitor__event-content {
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-1);
    flex: 1;
  }

  .execution-monitor__event-type {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary);
  }

  .execution-monitor__event-step {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-secondary);
  }

  .execution-monitor__event-message {
    font-size: var(--mcp-text-sm);
    color: var(--mcp-text-secondary);
  }

  /* Dark theme adjustments */
  [data-theme="dark"] .execution-monitor__operation-type {
    background: var(--mcp-primary-900);
    color: var(--mcp-primary-300);
  }

  [data-theme="dark"] .execution-monitor__event--step_started {
    background: rgba(var(--mcp-primary-500-rgb), 0.1);
  }

  [data-theme="dark"] .execution-monitor__event--step_completed,
  [data-theme="dark"] .execution-monitor__event--execution_completed {
    background: rgba(var(--mcp-success-500-rgb), 0.1);
  }

  [data-theme="dark"] .execution-monitor__event--step_failed,
  [data-theme="dark"] .execution-monitor__event--execution_failed {
    background: rgba(var(--mcp-error-500-rgb), 0.1);
  }

  /* Scrollbar */
  .execution-monitor__steps::-webkit-scrollbar,
  .execution-monitor__events::-webkit-scrollbar {
    width: 6px;
  }

  .execution-monitor__steps::-webkit-scrollbar-track,
  .execution-monitor__events::-webkit-scrollbar-track {
    background: transparent;
  }

  .execution-monitor__steps::-webkit-scrollbar-thumb,
  .execution-monitor__events::-webkit-scrollbar-thumb {
    background: var(--mcp-border-primary);
    border-radius: 3px;
  }

  .execution-monitor__steps::-webkit-scrollbar-thumb:hover,
  .execution-monitor__events::-webkit-scrollbar-thumb:hover {
    background: var(--mcp-border-secondary);
  }
</style>