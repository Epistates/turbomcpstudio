<!--
  MissionControl.svelte

  Unified workspace gestalt view coordinating:
  - Live protocol stream (left, primary)
  - AI copilot (right, contextual intelligence)
  - Coverage matrix (bottom-left, validation state)
  - Insights panel (bottom-right, emergent wisdom)

  Design principle: One continuous workflow, not separate tools.
  User observes → understands → validates → improves (all visible at once).
-->

<script lang="ts">
  import {
    workspaceStore,
    workspaceServerContext,
    workspaceSelectedMessage,
    workspaceSelectedTest,
    workspaceCoverage,
    workspaceInsights,
    workspaceSuggestions
  } from '$lib/stores/workspaceStore';
  import { contextStore } from '$lib/stores/contextStore';
  import ProtocolInspector from './ProtocolInspector.svelte';
  import CoverageMatrix from './gestalts/CoverageMatrix.svelte';
  import InsightsPanel from './gestalts/InsightsPanel.svelte';
  import AICopilot from './gestalts/AICopilot.svelte';
  import { createLogger } from '$lib/utils/logger';

  const logger = createLogger('MissionControl');

  // Reactive derived state from workspaceStore
  const context = $derived($workspaceServerContext);
  const selectedMessage = $derived($workspaceSelectedMessage);
  const selectedTest = $derived($workspaceSelectedTest);
  const coverage = $derived($workspaceCoverage);
  const insights = $derived($workspaceInsights);
  const suggestions = $derived($workspaceSuggestions);

  // Get current state values
  const currentState = $derived($workspaceStore);
  const isGeneratingTests = $derived(currentState.isGeneratingTests);
  const isRunningTests = $derived(currentState.isRunningTests);

  // Tab state for right panel
  let activeTab = $state<'copilot' | 'coverage' | 'insights'>('copilot');

  // Handlers for pane interactions
  function handleSelectMessage(messageId: string) {
    workspaceStore.selectMessage(messageId);
    logger.debug('Message selected via MissionControl:', messageId);
  }

  function handleSelectTest(testId: string) {
    workspaceStore.selectTest(testId);
    logger.debug('Test selected via MissionControl:', testId);
  }

  function handleSuggestionAction(suggestion: any) {
    logger.info('Executing suggestion:', suggestion.id);
    // Dispatch event or call action
    suggestion.action?.();
  }

  function handleInsightAction(insight: any) {
    logger.info('Addressing insight:', insight.type);
    // Dispatch event for insight-specific action
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

  <!-- Main workspace: Protocol (70%) + Context Panel (30%) -->
  <div class="workspace-container">
    <!-- Left: Live Protocol (Primary) -->
    <div class="protocol-column">
      <div class="pane-header compact">
        <h3>📊 Live Protocol</h3>
        <span class="message-badge">{currentState.protocolMessages.length}</span>
      </div>
      <div class="pane-content">
        <ProtocolInspector
          serverId={context.selectedServerId || ''}
          onSelectMessage={handleSelectMessage}
        />
      </div>
    </div>

    <!-- Right: Context Panel (Tabbed) -->
    <div class="context-panel">
      <!-- Tab bar -->
      <div class="tab-bar">
        <button
          class="tab"
          class:active={activeTab === 'copilot'}
          on:click={() => (activeTab = 'copilot')}
        >
          🤖
          <span class="tab-label">Copilot</span>
          <span class="tab-badge">{suggestions.length}</span>
        </button>
        <button
          class="tab"
          class:active={activeTab === 'coverage'}
          on:click={() => (activeTab = 'coverage')}
        >
          ✅
          <span class="tab-label">Coverage</span>
          <span class="tab-badge">{Math.round(coverage.overallPassRate)}%</span>
        </button>
        <button
          class="tab"
          class:active={activeTab === 'insights'}
          on:click={() => (activeTab = 'insights')}
        >
          📈
          <span class="tab-label">Insights</span>
          <span class="tab-badge">{insights.length}</span>
        </button>
      </div>

      <!-- Tab content -->
      <div class="tab-content">
        {#if activeTab === 'copilot'}
          <AICopilot
            selectedMessage={selectedMessage}
            selectedTest={selectedTest}
            suggestions={suggestions}
            insights={insights}
            onSuggestion={handleSuggestionAction}
            onInsight={handleInsightAction}
          />
        {:else if activeTab === 'coverage'}
          <CoverageMatrix
            coverage={coverage}
            selectedTestId={selectedTest?.id || null}
            onSelectTest={handleSelectTest}
          />
        {:else if activeTab === 'insights'}
          <InsightsPanel
            insights={insights}
            selectedTestId={selectedTest?.id || null}
            onSelectTest={handleSelectTest}
            onAction={handleInsightAction}
          />
        {/if}
      </div>
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

  /* Main workspace container (70/30 split) */
  .workspace-container {
    display: grid;
    grid-template-columns: 7fr 3fr;
    flex: 1;
    gap: 0;
    overflow: hidden;
    border-top: 1px solid var(--border-color);
  }

  /* Left column: Protocol Inspector */
  .protocol-column {
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border-color);
    min-height: 0;
  }

  /* Right column: Context Panel with tabs */
  .context-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  /* Tab bar */
  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
    padding: 0 0.5rem;
  }

  .tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem 0.5rem;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all 0.2s;
    white-space: nowrap;
  }

  .tab:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .tab.active {
    color: var(--info-text);
    border-bottom-color: var(--info-text);
  }

  .tab-label {
    display: none;
  }

  .tab-badge {
    display: inline-block;
    padding: 0.15rem 0.35rem;
    background: var(--bg-tertiary);
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .tab.active .tab-badge {
    background: var(--info-bg);
    color: var(--info-text);
  }

  /* Tab content area */
  .tab-content {
    flex: 1;
    overflow: auto;
    min-height: 0;
    padding: 0;
  }

  /* Pane header (both types) */
  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .pane-header.compact {
    padding: 0.5rem 0.75rem;
    gap: 0.5rem;
  }

  .pane-header h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .message-badge {
    display: inline-block;
    padding: 0.2rem 0.4rem;
    background: var(--bg-tertiary);
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .pane-content {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  /* Responsive adjustments */
  @media (max-width: 1440px) {
    .tab-label {
      display: inline;
    }

    .tab {
      padding: 0.75rem 0.75rem;
    }
  }

  @media (max-width: 1200px) {
    .workspace-container {
      grid-template-columns: 65% 35%;
    }
  }

  @media (max-width: 1024px) {
    .workspace-container {
      grid-template-columns: 60% 40%;
    }

    .tab-label {
      display: inline;
    }
  }

  @media (max-width: 768px) {
    .workspace-container {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
    }

    .protocol-column {
      border-right: none;
      border-bottom: 1px solid var(--border-color);
    }

    .context-panel {
      /* Full width on mobile */
    }

    .tab-label {
      display: inline;
    }

    .tab {
      padding: 0.5rem;
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
