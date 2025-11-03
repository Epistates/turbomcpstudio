<!--
  CoverageMatrix.svelte

  Shows test coverage as a matrix:
  Rows: Tools (one per row)
  Columns: Test categories (happy_path, edge_case, error, security, load)

  Each cell shows:
  - Status (untested=gray, passed=green, failed=red, running=yellow)
  - Test count
  - Click to view tests for that cell
-->

<script lang="ts">
  import { CheckCircle2, AlertCircle, Circle, Play } from 'lucide-svelte';
  import type { CoverageMatrix, ToolCoverage } from '$lib/stores/workspaceStore';

  interface Props {
    coverage: CoverageMatrix;
    selectedTestId: string | null;
    onSelectTest: (testId: string) => void;
  }

  const { coverage, selectedTestId, onSelectTest } = $props();

  const categories = [
    { key: 'happyPath', label: 'Happy Path', color: 'blue' },
    { key: 'edgeCase', label: 'Edge Case', color: 'purple' },
    { key: 'error', label: 'Error', color: 'red' },
    { key: 'security', label: 'Security', color: 'orange' },
    { key: 'load', label: 'Load', color: 'green' },
  ] as const;

  function getStatusIcon(status: string) {
    switch (status) {
      case 'passed':
        return CheckCircle2;
      case 'failed':
        return AlertCircle;
      case 'running':
        return Play;
      default:
        return Circle;
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'passed':
        return 'text-green-600 dark:text-green-400';
      case 'failed':
        return 'text-red-600 dark:text-red-400';
      case 'running':
        return 'text-yellow-600 dark:text-yellow-400';
      default:
        return 'text-gray-400 dark:text-gray-600';
    }
  }

  function getCellBackground(status: string): string {
    switch (status) {
      case 'passed':
        return 'bg-green-50 dark:bg-green-950 hover:bg-green-100 dark:hover:bg-green-900';
      case 'failed':
        return 'bg-red-50 dark:bg-red-950 hover:bg-red-100 dark:hover:bg-red-900';
      case 'running':
        return 'bg-yellow-50 dark:bg-yellow-950 hover:bg-yellow-100 dark:hover:bg-yellow-900';
      default:
        return 'bg-gray-50 dark:bg-gray-900 hover:bg-gray-100 dark:hover:bg-gray-800';
    }
  }
</script>

<div class="coverage-matrix">
  {#if coverage.totalTools === 0}
    <div class="empty-state">
      <Circle size={48} />
      <p>No tools available. Connect an MCP server first.</p>
    </div>
  {:else}
    <div class="matrix-container">
      <!-- Header row: category labels -->
      <div class="matrix-row header">
        <div class="tool-cell header-cell">Tool</div>
        {#each categories as category}
          <div class="coverage-cell header-cell">
            <span class="category-label">{category.label}</span>
          </div>
        {/each}
        <div class="coverage-cell header-cell">
          <span class="category-label">Total</span>
        </div>
      </div>

      <!-- Data rows: one per tool -->
      {#each Array.from(coverage.byTool.values()) as tool: ToolCoverage (tool.toolName)}
        {@const totalTests = (
          tool.happyPath.testIds.length +
          tool.edgeCase.testIds.length +
          tool.error.testIds.length +
          tool.security.testIds.length +
          tool.load.testIds.length
        )}
        {@const passedTests = [
          tool.happyPath,
          tool.edgeCase,
          tool.error,
          tool.security,
          tool.load,
        ].filter(c => c.status === 'passed').reduce((sum, c) => sum + c.testIds.length, 0)}

        <div class="matrix-row" class:has-failed={[
          tool.happyPath,
          tool.edgeCase,
          tool.error,
          tool.security,
          tool.load,
        ].some(c => c.status === 'failed')}>
          <div class="tool-cell">
            <span class="tool-name">{tool.toolName}</span>
          </div>

          {#each categories as category (category.key)}
            {@const cell = tool[category.key]}
            {@const Icon = getStatusIcon(cell.status)}

            <button
              class="coverage-cell {getCellBackground(cell.status)}"
              onclick={() => {
                if (cell.testIds.length > 0) {
                  onSelectTest(cell.testIds[0]);
                }
              }}
              title="{category.label}: {cell.status} ({cell.testIds.length} test{cell.testIds.length !== 1 ? 's' : ''})"
            >
              <Icon size={20} class={getStatusColor(cell.status)} />
              <span class="cell-count">{cell.testIds.length}</span>
            </button>
          {/each}

          <!-- Total column -->
          <div class="coverage-cell">
            <span class="cell-count">
              {passedTests}/{totalTests}
            </span>
          </div>
        </div>
      {/each}

      <!-- Summary row -->
      <div class="matrix-row summary">
        <div class="tool-cell">
          <span class="tool-name">Summary</span>
        </div>
        <div class="summary-cell">
          <span class="summary-label">
            {coverage.testedTools} of {coverage.totalTools} tools tested
          </span>
        </div>
        <div class="summary-cell">
          <span class="summary-label">
            {coverage.passedTests} of {coverage.totalTests} tests pass
          </span>
        </div>
        <div class="summary-cell">
          <span class="summary-label">
            {Math.round(coverage.overallPassRate)}% pass rate
          </span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .coverage-matrix {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: auto;
    padding: 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 2rem;
    color: var(--text-secondary);
    gap: 1rem;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.875rem;
  }

  .matrix-container {
    display: flex;
    flex-direction: column;
    padding: 0.75rem;
    gap: 0.5rem;
  }

  .matrix-row {
    display: grid;
    grid-template-columns: 160px repeat(5, 60px) 70px;
    gap: 0.5rem;
    align-items: center;
  }

  .matrix-row.has-failed {
    background: rgba(220, 38, 38, 0.05);
  }

  .matrix-row.summary {
    grid-template-columns: 160px 1fr;
    border-top: 2px solid var(--border-color);
    padding-top: 0.5rem;
    margin-top: 0.5rem;
  }

  .matrix-row.header {
    font-weight: 600;
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .tool-cell {
    padding: 0.5rem;
    font-size: 0.875rem;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tool-cell.header-cell {
    color: var(--text-secondary);
    background: transparent;
  }

  .tool-name {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .coverage-cell {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    padding: 0.5rem;
    border-radius: 0.375rem;
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.15s ease;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .coverage-cell.header-cell {
    cursor: default;
    color: var(--text-secondary);
    background: transparent;
    padding: 0.5rem 0.25rem;
  }

  .coverage-cell:not(.header-cell) {
    flex-direction: column;
    min-width: 60px;
    aspect-ratio: 1;
  }

  .coverage-cell:not(.header-cell):hover {
    border-color: var(--text-secondary);
    transform: scale(1.05);
  }

  .coverage-cell:disabled {
    cursor: default;
  }

  .category-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .cell-count {
    font-size: 0.75rem;
    font-weight: 600;
  }

  .summary-cell {
    display: flex;
    align-items: center;
    padding: 0.5rem;
    background: var(--bg-tertiary);
    border-radius: 0.375rem;
    font-size: 0.875rem;
  }

  .summary-label {
    display: block;
    font-weight: 500;
  }
</style>
