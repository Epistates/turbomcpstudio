<!--
  UnifiedTesting.svelte

  Unified testing workspace combining:
  - Test Catalog (browse, create, execute tests)
  - Coverage Matrix (visual tool coverage status)
  - Insights (patterns, recommendations)
  - AI Copilot (intelligent suggestions)

  Design principle: Complete testing workflow in one place.
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
  import TestCatalog from './TestCatalog.svelte';
  import CoverageMatrix from './gestalts/CoverageMatrix.svelte';
  import InsightsPanel from './gestalts/InsightsPanel.svelte';
  import AICopilot from './gestalts/AICopilot.svelte';
  import { createLogger } from '$lib/utils/logger';

  const logger = createLogger('UnifiedTesting');

  // Props
  const { serverId = null } = $props<{ serverId?: string | null }>();

  // Reactive derived state
  const context = $derived($workspaceServerContext);
  const selectedTest = $derived($workspaceSelectedTest);
  const coverage = $derived($workspaceCoverage);
  const insights = $derived($workspaceInsights);
  const suggestions = $derived($workspaceSuggestions);

  // Get current state
  const currentState = $derived($workspaceStore);
  const isGeneratingTests = $derived(currentState.isGeneratingTests);
  const isRunningTests = $derived(currentState.isRunningTests);

  // Active tab state
  let activeTab = $state<'catalog' | 'coverage' | 'insights' | 'copilot'>('catalog');

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

  // Get current server ID from context or props
  const currentServerId = $derived(serverId || context.selectedServerId || null);
</script>

<div class="unified-testing">
  <!-- Header with status -->
  <div class="testing-header">
    <div class="header-left">
      <h2>🧪 Testing</h2>
      {#if context.selectedServer}
        <span class="server-name">{context.selectedServer.config.name}</span>
      {/if}
    </div>
    <div class="header-status">
      {#if isGeneratingTests}
        <span class="status-badge generating">⏳ Generating...</span>
      {/if}
      {#if isRunningTests}
        <span class="status-badge running">▶ Running...</span>
      {/if}
    </div>
  </div>

  <!-- Tab navigation -->
  <div class="tab-navigation">
    <button
      class="nav-button"
      class:active={activeTab === 'catalog'}
      onclick={() => (activeTab = 'catalog')}
    >
      <span class="tab-icon">📋</span>
      <span class="tab-title">Test Catalog</span>
      <span class="tab-hint">Browse & Execute</span>
    </button>
    <button
      class="nav-button"
      class:active={activeTab === 'coverage'}
      onclick={() => (activeTab = 'coverage')}
    >
      <span class="tab-icon">✅</span>
      <span class="tab-title">Coverage</span>
      <span class="tab-hint">{coverage.testedTools}/{coverage.totalTools} tools</span>
    </button>
    <button
      class="nav-button"
      class:active={activeTab === 'insights'}
      onclick={() => (activeTab = 'insights')}
    >
      <span class="tab-icon">📈</span>
      <span class="tab-title">Insights</span>
      <span class="tab-hint">{insights.length} patterns</span>
    </button>
    <button
      class="nav-button"
      class:active={activeTab === 'copilot'}
      onclick={() => (activeTab = 'copilot')}
    >
      <span class="tab-icon">🤖</span>
      <span class="tab-title">AI Copilot</span>
      <span class="tab-hint">{suggestions.length} suggestions</span>
    </button>
  </div>

  <!-- Tab content -->
  <div class="tab-content-area">
    {#if activeTab === 'catalog'}
      <div class="tab-panel">
        <TestCatalog serverId={currentServerId} />
      </div>
    {:else if activeTab === 'coverage'}
      <div class="tab-panel">
        <CoverageMatrix
          coverage={coverage}
          selectedTestId={selectedTest?.id || null}
          onSelectTest={handleSelectTest}
        />
      </div>
    {:else if activeTab === 'insights'}
      <div class="tab-panel">
        <InsightsPanel
          insights={insights}
          selectedTestId={selectedTest?.id || null}
          onSelectTest={handleSelectTest}
          onAction={handleInsightAction}
        />
      </div>
    {:else if activeTab === 'copilot'}
      <div class="tab-panel">
        <AICopilot
          selectedMessage={null}
          selectedTest={selectedTest}
          suggestions={suggestions}
          insights={insights}
          onSuggestion={handleSuggestionAction}
          onInsight={handleInsightAction}
        />
      </div>
    {/if}
  </div>
</div>

<style>
  .unified-testing {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    overflow: hidden;
  }

  /* Header */
  .testing-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .testing-header h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .server-name {
    padding: 0.25rem 0.75rem;
    background: var(--bg-tertiary);
    border-radius: 0.375rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .header-status {
    display: flex;
    gap: 0.5rem;
  }

  .status-badge {
    padding: 0.35rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .status-badge.generating {
    background: var(--info-bg);
    color: var(--info-text);
  }

  .status-badge.running {
    background: var(--warning-bg);
    color: var(--warning-text);
  }

  /* Tab Navigation */
  .tab-navigation {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0;
    border-bottom: 2px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .nav-button {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    padding: 1rem;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-bottom: 3px solid transparent;
    transition: all 0.2s;
    position: relative;
    bottom: -2px;
  }

  .nav-button:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .nav-button.active {
    background: var(--bg-primary);
    color: var(--info-text);
    border-bottom-color: var(--info-text);
  }

  .tab-icon {
    font-size: 1.5rem;
    line-height: 1;
  }

  .tab-title {
    font-size: 0.875rem;
    font-weight: 600;
  }

  .tab-hint {
    font-size: 0.75rem;
    opacity: 0.7;
  }

  .nav-button.active .tab-hint {
    opacity: 1;
    color: var(--info-text);
  }

  /* Tab Content */
  .tab-content-area {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .tab-panel {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  /* Responsive */
  @media (max-width: 1024px) {
    .tab-navigation {
      grid-template-columns: repeat(2, 1fr);
    }

    .nav-button {
      padding: 0.75rem;
    }

    .tab-icon {
      font-size: 1.25rem;
    }

    .tab-title {
      font-size: 0.8rem;
    }
  }

  @media (max-width: 768px) {
    .testing-header {
      padding: 0.75rem 1rem;
    }

    .testing-header h2 {
      font-size: 1.1rem;
    }

    .server-name {
      font-size: 0.75rem;
    }

    .nav-button {
      padding: 0.5rem;
      gap: 0.15rem;
    }

    .tab-icon {
      font-size: 1.1rem;
    }

    .tab-title {
      font-size: 0.75rem;
    }

    .tab-hint {
      font-size: 0.7rem;
    }
  }

  /* CSS Variables - inherit from global or define fallbacks */
  :global(:root) {
    --bg-primary: #ffffff;
    --bg-secondary: #f9fafb;
    --bg-tertiary: #f3f4f6;
    --text-primary: #111827;
    --text-secondary: #6b7280;
    --border-color: #e5e7eb;
    --info-bg: #dbeafe;
    --info-text: #0284c7;
    --warning-bg: #fef3c7;
    --warning-text: #b45309;
  }

  :global(.dark) {
    --bg-primary: #0f172a;
    --bg-secondary: #1e293b;
    --bg-tertiary: #334155;
    --text-primary: #f1f5f9;
    --text-secondary: #cbd5e1;
    --border-color: #475569;
    --info-bg: #0c4a6e;
    --info-text: #7dd3fc;
    --warning-bg: #78350f;
    --warning-text: #fcd34d;
  }
</style>
