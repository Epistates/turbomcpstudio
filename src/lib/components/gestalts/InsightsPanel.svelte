<!--
  InsightsPanel.svelte

  Shows automatically detected patterns and insights:
  - Repeated failures
  - Slow responses
  - Untested tools
  - Missing edge cases

  Each insight is actionable with suggestions for resolution.
-->

<script lang="ts">
  import { AlertTriangle, AlertCircle, Info, Zap, TrendingUp } from 'lucide-svelte';
  import type { DetectedPattern } from '$lib/stores/workspaceStore';
  import { createLogger } from '$lib/utils/logger';

  interface Props {
    insights: DetectedPattern[];
    selectedTestId: string | null;
    onSelectTest: (testId: string) => void;
    onAction: (insight: DetectedPattern) => void;
  }

  const { insights, selectedTestId, onSelectTest, onAction } = $props();

  const logger = createLogger('InsightsPanel');

  function getInsightIcon(type: string) {
    switch (type) {
      case 'repeated_failure':
        return AlertTriangle;
      case 'slow_response':
        return TrendingUp;
      case 'untested_tool':
        return Info;
      case 'missing_edge_case':
        return AlertCircle;
      default:
        return AlertCircle;
    }
  }

  function getSeverityColor(severity: string): string {
    switch (severity) {
      case 'high':
        return 'text-red-600 dark:text-red-400';
      case 'medium':
        return 'text-yellow-600 dark:text-yellow-400';
      case 'low':
        return 'text-blue-600 dark:text-blue-400';
      default:
        return 'text-gray-600 dark:text-gray-400';
    }
  }

  function getBackgroundColor(severity: string): string {
    switch (severity) {
      case 'high':
        return 'bg-red-50 dark:bg-red-950 border-red-200 dark:border-red-800';
      case 'medium':
        return 'bg-yellow-50 dark:bg-yellow-950 border-yellow-200 dark:border-yellow-800';
      case 'low':
        return 'bg-blue-50 dark:bg-blue-950 border-blue-200 dark:border-blue-800';
      default:
        return 'bg-gray-50 dark:bg-gray-900 border-gray-200 dark:border-gray-800';
    }
  }

  function handleSelectRelatedTest(insight: DetectedPattern) {
    if (insight.relatedTestIds && insight.relatedTestIds.length > 0) {
      onSelectTest(insight.relatedTestIds[0]);
      logger.debug('Selected first related test:', insight.relatedTestIds[0]);
    }
  }

  function handleAction(insight: DetectedPattern) {
    logger.info('Insight action triggered:', insight.type);
    onAction(insight);
  }
</script>

<div class="insights-panel">
  {#if insights.length === 0}
    <div class="empty-state">
      <TrendingUp size={48} />
      <p>No issues detected. Keep testing!</p>
    </div>
  {:else}
    <div class="insights-list">
      {#each insights as insight (insight.type + insight.message)}
        {@const Icon = getInsightIcon(insight.type)}

        <div class="insight-card {getBackgroundColor(insight.severity)} border-l-4">
          <!-- Header: Icon + Title -->
          <div class="insight-header">
            <div class="insight-icon">
              <Icon size={20} class={getSeverityColor(insight.severity)} />
            </div>
            <div class="insight-title">
              <div class="title-text">{insight.message}</div>
              <div class="insight-type">{insight.type.replace(/_/g, ' ').toLowerCase()}</div>
            </div>
            <div class="severity-badge {insight.severity}">
              {insight.severity.charAt(0).toUpperCase()}
            </div>
          </div>

          <!-- Body: Description + Related Items -->
          {#if insight.relatedTestIds && insight.relatedTestIds.length > 0}
            <div class="insight-body">
              <div class="related-items">
                <span class="label">Related tests:</span>
                <div class="test-list">
                  {#each insight.relatedTestIds.slice(0, 3) as testId}
                    <button
                      class="test-link"
                      onclick={() => onSelectTest(testId)}
                      class:selected={testId === selectedTestId}
                    >
                      {testId.slice(0, 8)}...
                    </button>
                  {/each}
                  {#if insight.relatedTestIds.length > 3}
                    <span class="more-count">+{insight.relatedTestIds.length - 3}</span>
                  {/if}
                </div>
              </div>
            </div>
          {/if}

          <!-- Footer: Action -->
          {#if insight.suggestedAction}
            <div class="insight-footer">
              <button
                class="action-button"
                onclick={() => handleAction(insight)}
              >
                <Zap size={16} />
                {insight.suggestedAction}
              </button>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .insights-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: auto;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    color: var(--text-secondary);
    gap: 0.75rem;
    flex: 1;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.875rem;
  }

  .insights-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0.75rem;
    overflow: auto;
  }

  .insight-card {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem;
    border-radius: 0.5rem;
    border: 1px solid;
    border-left-width: 4px;
    transition: all 0.15s ease;
  }

  .insight-card:hover {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  }

  .insight-header {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .insight-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 0.125rem;
  }

  .insight-title {
    flex: 1;
    min-width: 0;
  }

  .title-text {
    font-weight: 600;
    font-size: 0.9375rem;
    margin: 0;
    color: var(--text-primary);
    word-break: break-word;
  }

  .insight-type {
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-top: 0.25rem;
  }

  .severity-badge {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 0.375rem;
    font-weight: 700;
    font-size: 0.75rem;
  }

  .severity-badge.high {
    background: rgba(220, 38, 38, 0.1);
    color: rgb(220, 38, 38);
  }

  .severity-badge.medium {
    background: rgba(180, 83, 9, 0.1);
    color: rgb(180, 83, 9);
  }

  .severity-badge.low {
    background: rgba(59, 130, 246, 0.1);
    color: rgb(59, 130, 246);
  }

  .insight-body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.5rem 0;
  }

  .related-items {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .label {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .test-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .test-link {
    display: inline-block;
    padding: 0.35rem 0.75rem;
    border-radius: 0.25rem;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-size: 0.8125rem;
    font-weight: 500;
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .test-link:hover {
    background: var(--bg-primary);
    border-color: var(--text-secondary);
  }

  .test-link.selected {
    background: var(--info-bg);
    color: var(--info-text);
    border-color: var(--info-text);
  }

  .more-count {
    display: inline-block;
    padding: 0.35rem 0.75rem;
    font-size: 0.8125rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .insight-footer {
    display: flex;
    gap: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid rgba(0, 0, 0, 0.05);
  }

  .action-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.8125rem;
    font-weight: 600;
    border: 1px solid var(--border-color);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-button:hover {
    background: var(--bg-secondary);
    border-color: var(--text-secondary);
  }

  .action-button:active {
    transform: scale(0.98);
  }

  /* Dark mode adjustments */
  :global(.dark) .insight-card:hover {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  :global(.dark) .insight-footer {
    border-top-color: rgba(255, 255, 255, 0.1);
  }
</style>
