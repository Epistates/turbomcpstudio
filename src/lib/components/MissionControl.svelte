<!--
  MissionControl.svelte

  High-level AI testing analysis dashboard:
  - Test Coverage Matrix (comprehensive tool testing status)
  - AI Copilot (intelligent suggestions based on testing patterns)
  - Insights Panel (detected patterns and recommendations)

  Design principle: Strategic testing overview, not operational protocol details.
  For detailed message inspection, use the Protocol tab.
-->

<script lang="ts">
  import {
    workspaceStore,
    workspaceServerContext,
    workspaceSelectedTest,
    workspaceCoverage,
    workspaceInsights,
    workspaceSuggestions
  } from '$lib/stores/workspaceStore';
  import { contextStore } from '$lib/stores/contextStore';
  import CoverageMatrix from './gestalts/CoverageMatrix.svelte';
  import InsightsPanel from './gestalts/InsightsPanel.svelte';
  import AICopilot from './gestalts/AICopilot.svelte';
  import { createLogger } from '$lib/utils/logger';

  const logger = createLogger('MissionControl');

  // Reactive derived state from workspaceStore
  const context = $derived($workspaceServerContext);
  const selectedTest = $derived($workspaceSelectedTest);
  const coverage = $derived($workspaceCoverage);
  const insights = $derived($workspaceInsights);
  const suggestions = $derived($workspaceSuggestions);

  // Get current state values
  const currentState = $derived($workspaceStore);
  const isGeneratingTests = $derived(currentState.isGeneratingTests);
  const isRunningTests = $derived(currentState.isRunningTests);

  // Tab state for main content
  let activeTab = $state<'coverage' | 'copilot' | 'insights'>('coverage');

  // Handlers
  function handleSelectTest(testId: string) {
    workspaceStore.selectTest(testId);
    logger.debug('Test selected:', testId);
  }

  function handleSuggestionAction(suggestion: any) {
    logger.info('Executing suggestion:', suggestion.id);
    suggestion.action?.();
  }

  function handleInsightAction(insight: any) {
    logger.info('Addressing insight:', insight.type);
  }
</script>

<div class="mission-control">
  <!-- Server context header -->
  <div class="header">
    <h2>
      {#if context.selectedServer}
        🚀 {context.selectedServer.config.name}
      {:else}
        🚀 No Server Selected
      {/if}
    </h2>
    <div class="status">
      {#if isGeneratingTests}
        <span class="badge generating">⏳ Generating tests...</span>
      {/if}
      {#if isRunningTests}
        <span class="badge running">▶ Running tests...</span>
      {/if}
      {#if context.connectionStatus === 'connected'}
        <span class="badge connected">✅ Connected</span>
      {:else if context.connectionStatus === 'connecting'}
        <span class="badge connecting">🔄 Connecting...</span>
      {:else}
        <span class="badge disconnected">⚠️ {context.connectionStatus}</span>
      {/if}
    </div>
  </div>

  <!-- Analysis Dashboard -->
  <div class="analysis-dashboard">
    <!-- Tab navigation -->
    <div class="tab-nav">
      <button
        type="button"
        class="nav-tab"
        class:active={activeTab === 'coverage'}
        onclick={() => (activeTab = 'coverage')}
      >
        <span class="tab-icon">✅</span>
        <span class="tab-label">Test Coverage</span>
        <span class="tab-badge">{coverage.testedTools}/{coverage.totalTools} tools</span>
      </button>
      <button
        type="button"
        class="nav-tab"
        class:active={activeTab === 'copilot'}
        onclick={() => (activeTab = 'copilot')}
      >
        <span class="tab-icon">🤖</span>
        <span class="tab-label">AI Copilot</span>
        <span class="tab-badge">{suggestions.length} suggestions</span>
      </button>
      <button
        type="button"
        class="nav-tab"
        class:active={activeTab === 'insights'}
        onclick={() => (activeTab = 'insights')}
      >
        <span class="tab-icon">📈</span>
        <span class="tab-label">Insights</span>
        <span class="tab-badge">{insights.length} patterns</span>
      </button>
    </div>

    <!-- Tab content -->
    <div class="tab-panel">
      {#if activeTab === 'coverage'}
        <div class="panel-content">
          <CoverageMatrix
            coverage={coverage}
            selectedTestId={selectedTest?.id || null}
            onSelectTest={handleSelectTest}
          />
        </div>
      {:else if activeTab === 'copilot'}
        <div class="panel-content">
          <AICopilot
            selectedMessage={null}
            selectedTest={selectedTest}
            suggestions={suggestions}
            insights={insights}
            onSuggestion={handleSuggestionAction}
            onInsight={handleInsightAction}
          />
        </div>
      {:else if activeTab === 'insights'}
        <div class="panel-content">
          <InsightsPanel
            insights={insights}
            selectedTestId={selectedTest?.id || null}
            onSelectTest={handleSelectTest}
            onAction={handleInsightAction}
          />
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .mission-control {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    color: var(--text-primary);
    gap: 0;
  }

  /* Header */
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
  }

  .header h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .status {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }

  .badge {
    display: inline-block;
    padding: 0.35rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .badge.connected {
    background: var(--success-bg);
    color: var(--success-text);
  }

  .badge.connecting {
    background: var(--warning-bg);
    color: var(--warning-text);
  }

  .badge.disconnected {
    background: var(--error-bg);
    color: var(--error-text);
  }

  .badge.generating,
  .badge.running {
    background: var(--info-bg);
    color: var(--info-text);
  }

  /* Analysis Dashboard */
  .analysis-dashboard {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  /* Tab Navigation */
  .tab-nav {
    display: flex;
    gap: 0;
    border-bottom: 2px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
    padding: 0 1rem;
  }

  .nav-tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 1rem 1.5rem;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 3px solid transparent;
    transition: all 0.2s;
    position: relative;
    bottom: -2px;
  }

  .nav-tab:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .nav-tab.active {
    color: var(--info-text);
    border-bottom-color: var(--info-text);
    background: var(--bg-primary);
  }

  .tab-icon {
    font-size: 1.25rem;
  }

  .tab-label {
    font-weight: 600;
  }

  .tab-badge {
    display: inline-block;
    padding: 0.2rem 0.5rem;
    background: var(--bg-tertiary);
    border-radius: 0.3rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .nav-tab.active .tab-badge {
    background: var(--info-bg);
    color: var(--info-text);
  }

  /* Tab Panel */
  .tab-panel {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .panel-content {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  /* Responsive adjustments */
  @media (max-width: 1024px) {
    .nav-tab {
      padding: 0.75rem 1rem;
      font-size: 0.85rem;
    }

    .tab-icon {
      font-size: 1.1rem;
    }
  }

  @media (max-width: 768px) {
    .tab-nav {
      padding: 0 0.5rem;
    }

    .nav-tab {
      flex-direction: column;
      padding: 0.5rem;
      gap: 0.25rem;
    }

    .tab-label {
      font-size: 0.75rem;
    }

    .tab-badge {
      font-size: 0.7rem;
    }
  }

  /* CSS Variables (fallback to light theme) */
  :global(:root) {
    --bg-primary: #ffffff;
    --bg-secondary: #f9fafb;
    --bg-tertiary: #f3f4f6;
    --text-primary: #111827;
    --text-secondary: #6b7280;
    --border-color: #e5e7eb;

    --success-bg: #ecfdf5;
    --success-text: #047857;
    --warning-bg: #fef3c7;
    --warning-text: #b45309;
    --error-bg: #fee2e2;
    --error-text: #dc2626;
    --info-bg: #dbeafe;
    --info-text: #0284c7;
  }

  /* Dark mode */
  :global(.dark) {
    --bg-primary: #0f172a;
    --bg-secondary: #1e293b;
    --bg-tertiary: #334155;
    --text-primary: #f1f5f9;
    --text-secondary: #cbd5e1;
    --border-color: #475569;

    --success-bg: #064e3b;
    --success-text: #86efac;
    --warning-bg: #78350f;
    --warning-text: #fcd34d;
    --error-bg: #7f1d1d;
    --error-text: #fca5a5;
    --info-bg: #0c4a6e;
    --info-text: #7dd3fc;
  }
</style>
