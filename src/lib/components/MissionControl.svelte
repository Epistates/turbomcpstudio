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

  <!-- Main grid: Protocol + Copilot (top), Coverage + Insights (bottom) -->
  <div class="workspace-grid">
    <!-- Top row: 60/40 split -->
    <div class="top-row">
      <!-- Left pane: Live Protocol Inspector (primary) -->
      <div class="pane protocol-pane">
        <div class="pane-header">
          <h3>📊 Live Protocol</h3>
          <div class="pane-controls">
            <span class="message-count">
              {$workspaceStore.getCurrentState().protocolMessages.length} messages
            </span>
          </div>
        </div>
        <div class="pane-content">
          <ProtocolInspector
            serverId={context.selectedServerId || ''}
            onSelectMessage={handleSelectMessage}
          />
        </div>
      </div>

      <!-- Right pane: AI Copilot (contextual intelligence) -->
      <div class="pane copilot-pane">
        <div class="pane-header">
          <h3>🤖 AI Copilot</h3>
          <div class="pane-controls">
            <span class="suggestion-count">
              {suggestions.length} suggestions
            </span>
          </div>
        </div>
        <div class="pane-content">
          <AICopilot
            selectedMessage={selectedMessage}
            selectedTest={selectedTest}
            suggestions={suggestions}
            insights={insights}
            onSuggestion={handleSuggestionAction}
            onInsight={handleInsightAction}
          />
        </div>
      </div>
    </div>

    <!-- Middle row: Coverage Matrix (full width) -->
    <div class="coverage-pane">
      <div class="pane-header">
        <h3>✅ Test Coverage</h3>
        <div class="pane-controls">
          <span class="coverage-stats">
            {Math.round(coverage.overallPassRate)}% pass · {coverage.testedTools}/{coverage.totalTools} tools tested
          </span>
        </div>
      </div>
      <div class="pane-content">
        <CoverageMatrix
          coverage={coverage}
          selectedTestId={selectedTest?.id || null}
          onSelectTest={handleSelectTest}
        />
      </div>
    </div>

    <!-- Bottom row: Insights (full width) -->
    <div class="insights-pane">
      <div class="pane-header">
        <h3>📈 Insights</h3>
        <div class="pane-controls">
          <span class="insight-count">
            {insights.length} patterns detected
          </span>
        </div>
      </div>
      <div class="pane-content">
        <InsightsPanel
          insights={insights}
          selectedTestId={selectedTest?.id || null}
          onSelectTest={handleSelectTest}
          onAction={handleInsightAction}
        />
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

  /* Workspace grid layout */
  .workspace-grid {
    display: grid;
    grid-template-rows: 1fr auto auto;
    grid-template-columns: 1fr;
    flex: 1;
    gap: 0;
    overflow: hidden;
  }

  /* Top row: Protocol + Copilot (60/40 split) */
  .top-row {
    display: grid;
    grid-template-columns: 1.5fr 1fr;
    gap: 0;
    border-bottom: 1px solid var(--border-color);
    min-height: 0;
  }

  /* Panes */
  .pane {
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border-color);
    min-height: 0;
  }

  .pane:last-child {
    border-right: none;
  }

  .protocol-pane {
    /* Left primary pane */
  }

  .copilot-pane {
    /* Right secondary pane */
    border-left: 1px solid var(--border-color);
    border-right: none;
  }

  .coverage-pane {
    border-bottom: 1px solid var(--border-color);
  }

  .insights-pane {
    /* Bottom pane */
  }

  /* Pane internals */
  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .pane-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .pane-controls {
    display: flex;
    gap: 0.75rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .message-count,
  .suggestion-count,
  .coverage-stats,
  .insight-count {
    display: inline-block;
    padding: 0.25rem 0.5rem;
    background: var(--bg-tertiary);
    border-radius: 0.25rem;
    font-weight: 500;
  }

  .pane-content {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  /* Responsive adjustments */
  @media (max-width: 1440px) {
    .top-row {
      grid-template-columns: 1.25fr 1fr;
    }
  }

  @media (max-width: 1024px) {
    .top-row {
      grid-template-columns: 1fr 1fr;
    }
  }

  @media (max-width: 768px) {
    .workspace-grid {
      grid-template-rows: 1fr auto auto auto;
    }

    .top-row {
      grid-template-columns: 1fr;
    }

    .pane {
      border-right: none;
      border-bottom: 1px solid var(--border-color);
    }

    .copilot-pane {
      border-left: none;
    }

    .coverage-pane {
      border-bottom: 1px solid var(--border-color);
    }

    .insights-pane {
      border-bottom: none;
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
