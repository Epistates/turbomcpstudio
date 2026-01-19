<!--
  JSON Diff Viewer Component
  Compares two JSON objects and highlights differences
-->
<script lang="ts">
  import { ChevronRight, ChevronDown, Plus, Minus, Edit3 } from 'lucide-svelte';

  interface Props {
    left: any;
    right: any;
    leftLabel?: string;
    rightLabel?: string;
    expanded?: boolean;
  }

  let { left, right, leftLabel = 'Before', rightLabel = 'After', expanded = true }: Props = $props();

  // Diff types
  type DiffType = 'added' | 'removed' | 'modified' | 'unchanged';

  interface DiffNode {
    key: string;
    type: DiffType;
    leftValue?: any;
    rightValue?: any;
    children?: DiffNode[];
  }

  // Compute diff between two values
  function computeDiff(leftVal: any, rightVal: any, key: string = 'root'): DiffNode {
    const leftType = getType(leftVal);
    const rightType = getType(rightVal);

    // Both undefined/null
    if (leftVal === undefined && rightVal === undefined) {
      return { key, type: 'unchanged', leftValue: leftVal, rightValue: rightVal };
    }

    // Added (left undefined, right has value)
    if (leftVal === undefined && rightVal !== undefined) {
      return { key, type: 'added', rightValue: rightVal };
    }

    // Removed (left has value, right undefined)
    if (leftVal !== undefined && rightVal === undefined) {
      return { key, type: 'removed', leftValue: leftVal };
    }

    // Different types
    if (leftType !== rightType) {
      return { key, type: 'modified', leftValue: leftVal, rightValue: rightVal };
    }

    // Both objects
    if (leftType === 'object' && rightType === 'object') {
      const allKeys = new Set([...Object.keys(leftVal || {}), ...Object.keys(rightVal || {})]);
      const children: DiffNode[] = [];
      let hasChanges = false;

      for (const k of allKeys) {
        const child = computeDiff(leftVal?.[k], rightVal?.[k], k);
        children.push(child);
        if (child.type !== 'unchanged') hasChanges = true;
      }

      return {
        key,
        type: hasChanges ? 'modified' : 'unchanged',
        leftValue: leftVal,
        rightValue: rightVal,
        children
      };
    }

    // Both arrays
    if (leftType === 'array' && rightType === 'array') {
      const maxLen = Math.max(leftVal.length, rightVal.length);
      const children: DiffNode[] = [];
      let hasChanges = false;

      for (let i = 0; i < maxLen; i++) {
        const child = computeDiff(leftVal[i], rightVal[i], `[${i}]`);
        children.push(child);
        if (child.type !== 'unchanged') hasChanges = true;
      }

      return {
        key,
        type: hasChanges ? 'modified' : 'unchanged',
        leftValue: leftVal,
        rightValue: rightVal,
        children
      };
    }

    // Primitives
    if (leftVal === rightVal) {
      return { key, type: 'unchanged', leftValue: leftVal, rightValue: rightVal };
    }

    return { key, type: 'modified', leftValue: leftVal, rightValue: rightVal };
  }

  function getType(val: any): string {
    if (val === null) return 'null';
    if (val === undefined) return 'undefined';
    if (Array.isArray(val)) return 'array';
    return typeof val;
  }

  function formatValue(val: any): string {
    if (val === null) return 'null';
    if (val === undefined) return 'undefined';
    if (typeof val === 'string') return `"${val}"`;
    if (typeof val === 'object') return JSON.stringify(val, null, 2);
    return String(val);
  }

  // State for expanded nodes
  let expandedNodes = $state<Set<string>>(new Set(['root']));

  function toggleNode(path: string) {
    if (expandedNodes.has(path)) {
      expandedNodes.delete(path);
    } else {
      expandedNodes.add(path);
    }
    expandedNodes = new Set(expandedNodes);
  }

  // Compute the diff tree
  const diffTree = $derived(computeDiff(left, right));

  // Count changes
  function countChanges(node: DiffNode): { added: number; removed: number; modified: number } {
    let result = { added: 0, removed: 0, modified: 0 };

    if (node.type === 'added') result.added++;
    else if (node.type === 'removed') result.removed++;
    else if (node.type === 'modified' && !node.children) result.modified++;

    if (node.children) {
      for (const child of node.children) {
        const childCounts = countChanges(child);
        result.added += childCounts.added;
        result.removed += childCounts.removed;
        result.modified += childCounts.modified;
      }
    }

    return result;
  }

  const changeCounts = $derived(countChanges(diffTree));
</script>

<div class="json-diff-viewer">
  <!-- Header with stats -->
  <div class="diff-header">
    <div class="diff-labels">
      <span class="diff-label left">{leftLabel}</span>
      <span class="diff-arrow">→</span>
      <span class="diff-label right">{rightLabel}</span>
    </div>
    <div class="diff-stats">
      {#if changeCounts.added > 0}
        <span class="diff-stat added">
          <Plus size={12} />
          {changeCounts.added} added
        </span>
      {/if}
      {#if changeCounts.removed > 0}
        <span class="diff-stat removed">
          <Minus size={12} />
          {changeCounts.removed} removed
        </span>
      {/if}
      {#if changeCounts.modified > 0}
        <span class="diff-stat modified">
          <Edit3 size={12} />
          {changeCounts.modified} modified
        </span>
      {/if}
      {#if changeCounts.added === 0 && changeCounts.removed === 0 && changeCounts.modified === 0}
        <span class="diff-stat unchanged">No differences</span>
      {/if}
    </div>
  </div>

  <!-- Diff tree -->
  <div class="diff-content">
    {#snippet renderNode(node: DiffNode, path: string, depth: number)}
      {@const isExpanded = expandedNodes.has(path)}
      {@const hasChildren = node.children && node.children.length > 0}
      {@const indent = depth * 16}

      <div class="diff-node {node.type}" style="padding-left: {indent}px">
        {#if hasChildren}
          <button class="diff-toggle" onclick={() => toggleNode(path)}>
            {#if isExpanded}
              <ChevronDown size={14} />
            {:else}
              <ChevronRight size={14} />
            {/if}
          </button>
        {:else}
          <span class="diff-toggle-placeholder"></span>
        {/if}

        <span class="diff-key">{node.key}:</span>

        {#if node.type === 'added'}
          <span class="diff-icon added"><Plus size={12} /></span>
          <span class="diff-value added">{formatValue(node.rightValue)}</span>
        {:else if node.type === 'removed'}
          <span class="diff-icon removed"><Minus size={12} /></span>
          <span class="diff-value removed">{formatValue(node.leftValue)}</span>
        {:else if node.type === 'modified' && !hasChildren}
          <span class="diff-icon modified"><Edit3 size={12} /></span>
          <span class="diff-value removed">{formatValue(node.leftValue)}</span>
          <span class="diff-arrow-inline">→</span>
          <span class="diff-value added">{formatValue(node.rightValue)}</span>
        {:else if !hasChildren}
          <span class="diff-value unchanged">{formatValue(node.leftValue)}</span>
        {/if}
      </div>

      {#if hasChildren && isExpanded && node.children}
        {#each node.children as child (child.key)}
          {@render renderNode(child, `${path}.${child.key}`, depth + 1)}
        {/each}
      {/if}
    {/snippet}

    {#if diffTree.children && diffTree.children.length > 0}
      {#each diffTree.children as child (child.key)}
        {@render renderNode(child, child.key, 0)}
      {/each}
    {:else}
      {@render renderNode(diffTree, 'root', 0)}
    {/if}
  </div>
</div>

<style>
  .json-diff-viewer {
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 12px;
    background: var(--mcp-surface-secondary);
    border-radius: var(--mcp-radius-lg);
    overflow: hidden;
  }

  .diff-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--mcp-space-3) var(--mcp-space-4);
    background: var(--mcp-surface-tertiary);
    border-bottom: 1px solid var(--mcp-border-primary);
  }

  .diff-labels {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .diff-label {
    padding: 2px 8px;
    border-radius: var(--mcp-radius-sm);
    font-size: 11px;
    font-weight: 600;
  }

  .diff-label.left {
    background: var(--mcp-error-100);
    color: var(--mcp-error-700);
  }

  .diff-label.right {
    background: var(--mcp-success-100);
    color: var(--mcp-success-700);
  }

  .diff-arrow {
    color: var(--mcp-text-tertiary);
  }

  .diff-stats {
    display: flex;
    gap: var(--mcp-space-3);
  }

  .diff-stat {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 500;
  }

  .diff-stat.added {
    color: var(--mcp-success-600);
  }

  .diff-stat.removed {
    color: var(--mcp-error-600);
  }

  .diff-stat.modified {
    color: var(--mcp-warning-600);
  }

  .diff-stat.unchanged {
    color: var(--mcp-text-tertiary);
  }

  .diff-content {
    padding: var(--mcp-space-3);
    max-height: 400px;
    overflow-y: auto;
  }

  .diff-node {
    display: flex;
    align-items: flex-start;
    gap: 4px;
    padding: 2px 0;
    line-height: 1.4;
  }

  .diff-node.added {
    background: rgba(34, 197, 94, 0.1);
  }

  .diff-node.removed {
    background: rgba(239, 68, 68, 0.1);
  }

  .diff-node.modified {
    background: rgba(234, 179, 8, 0.05);
  }

  .diff-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--mcp-text-tertiary);
    cursor: pointer;
    flex-shrink: 0;
  }

  .diff-toggle:hover {
    color: var(--mcp-text-primary);
  }

  .diff-toggle-placeholder {
    width: 16px;
    flex-shrink: 0;
  }

  .diff-key {
    color: var(--mcp-primary-600);
    font-weight: 500;
  }

  .diff-icon {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .diff-icon.added {
    color: var(--mcp-success-600);
  }

  .diff-icon.removed {
    color: var(--mcp-error-600);
  }

  .diff-icon.modified {
    color: var(--mcp-warning-600);
  }

  .diff-value {
    word-break: break-all;
  }

  .diff-value.added {
    color: var(--mcp-success-700);
    background: rgba(34, 197, 94, 0.15);
    padding: 0 4px;
    border-radius: 2px;
  }

  .diff-value.removed {
    color: var(--mcp-error-700);
    background: rgba(239, 68, 68, 0.15);
    padding: 0 4px;
    border-radius: 2px;
    text-decoration: line-through;
  }

  .diff-value.unchanged {
    color: var(--mcp-text-secondary);
  }

  .diff-arrow-inline {
    color: var(--mcp-text-tertiary);
    margin: 0 4px;
  }

  /* Dark mode */
  [data-theme="dark"] .diff-label.left {
    background: rgba(239, 68, 68, 0.2);
    color: var(--mcp-error-400);
  }

  [data-theme="dark"] .diff-label.right {
    background: rgba(34, 197, 94, 0.2);
    color: var(--mcp-success-400);
  }

  [data-theme="dark"] .diff-value.added {
    color: var(--mcp-success-400);
    background: rgba(34, 197, 94, 0.2);
  }

  [data-theme="dark"] .diff-value.removed {
    color: var(--mcp-error-400);
    background: rgba(239, 68, 68, 0.2);
  }

  /* Scrollbar */
  .diff-content::-webkit-scrollbar {
    width: 6px;
  }

  .diff-content::-webkit-scrollbar-track {
    background: transparent;
  }

  .diff-content::-webkit-scrollbar-thumb {
    background: var(--mcp-border-primary);
    border-radius: 3px;
  }
</style>
