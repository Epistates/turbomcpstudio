<script lang="ts">
  import { ChevronDown, ChevronRight, Copy, CheckCircle, AlertTriangle, Info, AlertCircle } from 'lucide-svelte';
  import ResourceLink from './ResourceLink.svelte';
  import { parseToolResult, getConformanceSummary, type CallToolResult, type ParseResult } from '$lib/types/mcp-parser';

  /**
   * StructuredContentViewer - Testing Tool Edition
   *
   * Philosophy: PERMISSIVE display with DETAILED observability
   * - Accepts ANY response format
   * - Shows conformance to MCP 2025-06-18
   * - Provides raw + parsed dual view
   * - Makes protocol deviations visible but non-blocking
   */

  // Props
  const {
    result,
    serverId,
    onCopy = () => {}
  }: {
    result: any;  // Intentionally permissive - accept anything!
    serverId?: string;
    onCopy?: (text: string) => void;
  } = $props();

  // State
  let viewMode = $state<'parsed' | 'raw' | 'split'>('parsed');
  let expandedPaths = $state<Set<string>>(new Set());
  let showSchema = $state(false);
  let showConformanceDetails = $state(false);

  // Parse result with conformance tracking
  const parseResult = $derived<ParseResult<CallToolResult> | null>(
    result ? parseToolResult(result) : null
  );

  // Derived state
  const hasStructuredContent = $derived(
    parseResult?.parsed?.structuredContent !== undefined &&
    parseResult?.parsed?.structuredContent !== null
  );
  const hasSchema = $derived(parseResult?.parsed && 'outputSchema' in result);
  const hasTextContent = $derived(
    parseResult?.parsed?.content && parseResult.parsed.content.length > 0
  );
  const conformanceSummary = $derived(
    parseResult ? getConformanceSummary(parseResult.conformance) : ''
  );

  // Count issues by severity
  const issueCount = $derived({
    errors: parseResult?.conformance.issues.filter(i => i.severity === 'error').length ?? 0,
    warnings: parseResult?.conformance.issues.filter(i => i.severity === 'warning').length ?? 0,
    info: parseResult?.conformance.issues.filter(i => i.severity === 'info').length ?? 0
  });

  function togglePath(path: string) {
    if (expandedPaths.has(path)) {
      expandedPaths.delete(path);
    } else {
      expandedPaths.add(path);
    }
    expandedPaths = new Set(expandedPaths); // Trigger reactivity
  }

  function isExpanded(path: string): boolean {
    return expandedPaths.has(path);
  }

  function formatJson(obj: any): string {
    return JSON.stringify(obj, null, 2);
  }

  function getTypeColor(type: string): string {
    const colors: Record<string, string> = {
      string: 'text-green-600 dark:text-green-400',
      number: 'text-blue-600 dark:text-blue-400',
      boolean: 'text-purple-600 dark:text-purple-400',
      object: 'text-orange-600 dark:text-orange-400',
      array: 'text-pink-600 dark:text-pink-400',
      null: 'text-gray-500 dark:text-gray-400',
    };
    return colors[type] || 'text-gray-700 dark:text-gray-300';
  }

  function getValueType(value: any): string {
    if (value === null) return 'null';
    if (Array.isArray(value)) return 'array';
    return typeof value;
  }

  function renderValue(value: any, path: string = '', level: number = 0): any {
    const type = getValueType(value);
    const indent = level * 20;

    if (type === 'object') {
      const keys = Object.keys(value);
      const expanded = isExpanded(path);

      return {
        type: 'object',
        expanded,
        keys,
        path,
        indent,
        value
      };
    } else if (type === 'array') {
      const expanded = isExpanded(path);

      return {
        type: 'array',
        expanded,
        length: value.length,
        path,
        indent,
        value
      };
    } else {
      // Primitive value
      return {
        type: 'primitive',
        valueType: type,
        value,
        indent
      };
    }
  }

  function validateAgainstSchema(data: any, schema: any): { valid: boolean; errors: string[] } {
    // Basic schema validation (simplified for MVP)
    const errors: string[] = [];

    if (schema.type === 'object' && schema.properties) {
      if (typeof data !== 'object' || data === null) {
        errors.push('Expected object');
        return { valid: false, errors };
      }

      // Check required fields
      if (schema.required) {
        for (const field of schema.required) {
          if (!(field in data)) {
            errors.push(`Missing required field: ${field}`);
          }
        }
      }

      // Check property types
      for (const [key, propSchema] of Object.entries(schema.properties)) {
        if (key in data) {
          const propType = (propSchema as any).type;
          const actualType = getValueType(data[key]);

          if (propType === 'number' && actualType !== 'number') {
            errors.push(`Field '${key}' should be number, got ${actualType}`);
          } else if (propType === 'string' && actualType !== 'string') {
            errors.push(`Field '${key}' should be string, got ${actualType}`);
          } else if (propType === 'boolean' && actualType !== 'boolean') {
            errors.push(`Field '${key}' should be boolean, got ${actualType}`);
          }
        }
      }
    }

    return { valid: errors.length === 0, errors };
  }

  const validation = $derived(() => {
    if (!hasStructuredContent || !hasSchema || !result) {
      return null;
    }
    return validateAgainstSchema(result.structuredContent, result.outputSchema);
  });
</script>

{#if !result}
  <div class="text-center py-8">
    <p class="text-sm text-gray-600 dark:text-gray-400">No result to display</p>
  </div>
{:else}
  <div class="flex flex-col h-full">
    <!-- View Mode Selector & Conformance Status -->
    <div class="flex items-center justify-between px-4 py-2 bg-gray-50 dark:bg-gray-800/50 border-b border-gray-200 dark:border-gray-700">
      <div class="flex items-center gap-3">
        <!-- View Mode Toggle -->
        <div class="flex items-center gap-1 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg p-1">
          <button
            onclick={() => viewMode = 'parsed'}
            class="px-3 py-1 text-xs font-medium rounded transition-colors {viewMode === 'parsed' ? 'bg-blue-600 text-white' : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'}"
          >
            Parsed
          </button>
          <button
            onclick={() => viewMode = 'raw'}
            class="px-3 py-1 text-xs font-medium rounded transition-colors {viewMode === 'raw' ? 'bg-blue-600 text-white' : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'}"
          >
            Raw JSON
          </button>
          <button
            onclick={() => viewMode = 'split'}
            class="px-3 py-1 text-xs font-medium rounded transition-colors {viewMode === 'split' ? 'bg-blue-600 text-white' : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'}"
          >
            Split
          </button>
        </div>

        <!-- Protocol Conformance Status -->
        {#if parseResult}
          {@const conf = parseResult.conformance}
          <button
            onclick={() => showConformanceDetails = !showConformanceDetails}
            class="flex items-center gap-1 px-2 py-1 text-xs rounded border transition-colors {
              conf.isCompliant
                ? 'border-green-200 bg-green-50 text-green-700 dark:border-green-800 dark:bg-green-900/20 dark:text-green-300'
                : issueCount.errors > 0
                  ? 'border-red-200 bg-red-50 text-red-700 dark:border-red-800 dark:bg-red-900/20 dark:text-red-300'
                  : issueCount.warnings > 0
                    ? 'border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-300'
                    : 'border-blue-200 bg-blue-50 text-blue-700 dark:border-blue-800 dark:bg-blue-900/20 dark:text-blue-300'
            } hover:opacity-80"
          >
            {#if conf.isCompliant}
              <CheckCircle size={12} />
              <span>MCP 2025-06-18</span>
            {:else if issueCount.errors > 0}
              <AlertCircle size={12} />
              <span>{issueCount.errors} error{issueCount.errors > 1 ? 's' : ''}</span>
            {:else if issueCount.warnings > 0}
              <AlertTriangle size={12} />
              <span>{issueCount.warnings} warning{issueCount.warnings > 1 ? 's' : ''}</span>
            {:else}
              <AlertCircle size={12} />
              <span>Unknown Format</span>
            {/if}
          </button>
        {/if}

        <!-- Schema Validation Status (if present) -->
        {#if hasStructuredContent && hasSchema && validation}
          {@const v = validation()}
          {#if v}
            <div class="flex items-center gap-1 text-xs {v.valid ? 'text-green-600 dark:text-green-400' : 'text-amber-600 dark:text-amber-400'}">
              {#if v.valid}
                <CheckCircle size={14} />
                <span>Schema Valid</span>
              {:else}
                <AlertTriangle size={14} />
                <span>{v.errors.length} schema error{v.errors.length > 1 ? 's' : ''}</span>
              {/if}
            </div>
          {/if}
        {/if}
      </div>

      <div class="flex items-center gap-2">
        {#if hasSchema}
          <button
            onclick={() => showSchema = !showSchema}
            class="text-xs text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 flex items-center gap-1"
          >
            {showSchema ? 'Hide' : 'Show'} Schema
            {#if showSchema}
              <ChevronDown size={12} />
            {:else}
              <ChevronRight size={12} />
            {/if}
          </button>
        {/if}
        <button
          onclick={() => onCopy(formatJson(result))}
          class="p-1 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
          title="Copy full result"
        >
          <Copy size={14} />
        </button>
      </div>
    </div>

    <!-- Conformance Details Panel -->
    {#if showConformanceDetails && parseResult}
      <div class="px-4 py-3 border-b {
        parseResult.conformance.isCompliant
          ? 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800'
          : issueCount.errors > 0
            ? 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800'
            : 'bg-amber-50 dark:bg-amber-900/20 border-amber-200 dark:border-amber-800'
      }">
        <div class="text-xs">
          <div class="font-semibold mb-2 {
            parseResult.conformance.isCompliant
              ? 'text-green-800 dark:text-green-200'
              : issueCount.errors > 0
                ? 'text-red-800 dark:text-red-200'
                : 'text-amber-800 dark:text-amber-200'
          }">
            Protocol Conformance: {parseResult.conformance.version}
          </div>

          <p class="mb-2 {
            parseResult.conformance.isCompliant
              ? 'text-green-700 dark:text-green-300'
              : issueCount.errors > 0
                ? 'text-red-700 dark:text-red-300'
                : 'text-amber-700 dark:text-amber-300'
          }">
            {conformanceSummary}
          </p>

          {#if parseResult.conformance.issues.length > 0}
            <details class="mt-2">
              <summary class="cursor-pointer font-medium {
                parseResult.conformance.isCompliant
                  ? 'text-green-800 dark:text-green-200'
                  : issueCount.errors > 0
                    ? 'text-red-800 dark:text-red-200'
                    : 'text-amber-800 dark:text-amber-200'
              }">
                {parseResult.conformance.issues.length} issue{parseResult.conformance.issues.length > 1 ? 's' : ''} detected
              </summary>
              <ul class="mt-2 space-y-1 {
                parseResult.conformance.isCompliant
                  ? 'text-green-700 dark:text-green-300'
                  : issueCount.errors > 0
                    ? 'text-red-700 dark:text-red-300'
                    : 'text-amber-700 dark:text-amber-300'
              }">
                {#each parseResult.conformance.issues as issue}
                  <li class="flex items-start gap-2">
                    <span class="font-semibold uppercase text-xs">
                      {issue.severity}:
                    </span>
                    <div class="flex-1">
                      {#if issue.field}
                        <code class="bg-black/10 dark:bg-white/10 px-1 rounded">{issue.field}</code>
                      {/if}
                      {issue.message}
                      {#if issue.expected}
                        <div class="mt-1 text-xs">
                          Expected: <code class="bg-black/10 dark:bg-white/10 px-1 rounded">{issue.expected}</code>
                        </div>
                      {/if}
                      {#if issue.received}
                        <div class="mt-1 text-xs">
                          Received: <code class="bg-black/10 dark:bg-white/10 px-1 rounded">{issue.received}</code>
                        </div>
                      {/if}
                    </div>
                  </li>
                {/each}
              </ul>
            </details>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Schema Display -->
    {#if showSchema && hasSchema}
      <div class="px-4 py-3 bg-blue-50 dark:bg-blue-900/20 border-b border-blue-200 dark:border-blue-800">
        <div class="text-xs font-mono">
          <div class="text-blue-800 dark:text-blue-200 font-semibold mb-1">Output Schema:</div>
          <pre class="text-blue-700 dark:text-blue-300 whitespace-pre-wrap">{formatJson(result.outputSchema)}</pre>
        </div>
      </div>
    {/if}

    <!-- Validation Errors -->
    {#if hasStructuredContent && hasSchema && validation}
      {@const v = validation()}
      {#if v && !v.valid}
        <div class="px-4 py-3 bg-amber-50 dark:bg-amber-900/20 border-b border-amber-200 dark:border-amber-800">
          <div class="text-xs">
            <div class="flex items-center gap-1 text-amber-800 dark:text-amber-200 font-semibold mb-2">
              <AlertTriangle size={14} />
              Validation Errors:
            </div>
            <ul class="space-y-1 text-amber-700 dark:text-amber-300">
              {#each v.errors as error}
                <li>• {error}</li>
              {/each}
            </ul>
          </div>
        </div>
      {/if}
    {/if}

    <!-- Content Display -->
    <div class="flex-1 overflow-hidden min-h-0">
      {#if viewMode === 'split'}
        <!-- Split View: Parsed + Raw side by side -->
        <div class="flex h-full">
          <div class="flex-1 overflow-y-auto border-r border-gray-200 dark:border-gray-700">
            {@render parsedView()}
          </div>
          <div class="flex-1 overflow-y-auto">
            {@render rawView()}
          </div>
        </div>
      {:else if viewMode === 'parsed'}
        <!-- Parsed View Only -->
        <div class="h-full overflow-y-auto">
          {@render parsedView()}
        </div>
      {:else}
        <!-- Raw View Only -->
        <div class="h-full overflow-y-auto">
          {@render rawView()}
        </div>
      {/if}
    </div>
  </div>
{/if}

{#snippet parsedView()}
  <div class="p-4">
    <h4 class="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-3">Parsed Content</h4>

    {#if parseResult?.parsed?.content && parseResult.parsed.content.length > 0}
      <div class="space-y-3">
        {#each parseResult.parsed.content as item, index}
          {#if item.type === 'text'}
            <div class="bg-gray-50 dark:bg-gray-800 p-3 rounded-lg">
              <div class="text-xs text-gray-500 dark:text-gray-400 mb-1">Text Content</div>
              <pre class="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap font-sans">{item.text}</pre>
            </div>
          {:else if item.type === 'image'}
            <div class="bg-gray-50 dark:bg-gray-800 p-3 rounded-lg">
              <div class="text-xs text-gray-600 dark:text-gray-400 mb-2">
                Image: {item.mimeType || 'unknown'}
              </div>
              <img src={item.data} alt="" class="max-w-full rounded" />
            </div>
          {:else if item.type === 'audio'}
            <div class="bg-gray-50 dark:bg-gray-800 p-3 rounded-lg">
              <div class="text-xs text-gray-600 dark:text-gray-400 mb-2">
                Audio: {item.mimeType || 'unknown'}
              </div>
              <audio src={item.data} controls class="w-full" />
            </div>
          {:else if item.type === 'resource_link'}
            <ResourceLink
              uri={item.uri || ''}
              name={item.name}
              description={item.description}
              mimeType={item.mimeType}
              serverId={serverId}
            />
          {:else if item.type === 'resource'}
            <div class="bg-gray-50 dark:bg-gray-800 p-3 rounded-lg">
              <div class="text-xs text-gray-600 dark:text-gray-400 mb-2">
                Embedded Resource: {item.resource?.uri || 'unknown'}
              </div>
              {#if item.resource?.text}
                <pre class="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap font-mono">{item.resource.text}</pre>
              {:else if item.resource?.blob}
                <div class="text-xs text-gray-500 dark:text-gray-400">Binary data ({item.resource.blob.length} bytes)</div>
              {/if}
            </div>
          {:else}
            <div class="bg-amber-50 dark:bg-amber-900/20 p-3 rounded-lg border border-amber-200 dark:border-amber-800">
              <div class="text-xs text-amber-700 dark:text-amber-300">
                Unknown content type: {item.type}
              </div>
              <pre class="text-xs mt-2 text-amber-600 dark:text-amber-400">{formatJson(item)}</pre>
            </div>
          {/if}
        {/each}
      </div>
    {:else}
      <div class="text-center py-8">
        <AlertTriangle size={32} class="mx-auto text-gray-400 dark:text-gray-500 mb-2" />
        <p class="text-sm text-gray-600 dark:text-gray-400">
          No parseable content found
        </p>
        <p class="text-xs text-gray-500 dark:text-gray-500 mt-1">
          Switch to Raw view to see the response
        </p>
      </div>
    {/if}

    <!-- Structured Content (if present) -->
    {#if hasStructuredContent && parseResult?.parsed?.structuredContent}
      <div class="mt-6">
        <h4 class="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-2">Structured Data</h4>
        <div class="bg-gray-900 dark:bg-gray-950 text-gray-100 p-4 rounded-lg font-mono text-sm">
          {@render structuredDataTree(parseResult.parsed.structuredContent, '', 0)}
        </div>
      </div>
    {/if}
  </div>
{/snippet}

{#snippet rawView()}
  <div class="p-4">
    <h4 class="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-3">Raw JSON Response</h4>
    <div class="bg-gray-900 dark:bg-gray-950 text-gray-100 p-4 rounded-lg font-mono text-xs overflow-x-auto">
      <pre class="whitespace-pre-wrap">{formatJson(parseResult?.raw ?? result)}</pre>
    </div>
  </div>
{/snippet}

{#snippet structuredDataTree(data: any, path: string, level: number)}
  {@const rendered = renderValue(data, path, level)}

  {#if rendered.type === 'object'}
    <div style="padding-left: {rendered.indent}px">
      <button
        onclick={() => togglePath(rendered.path)}
        class="flex items-center gap-1 hover:bg-gray-800 dark:hover:bg-gray-900 px-1 rounded"
      >
        {#if rendered.expanded}
          <ChevronDown size={14} class="text-gray-400" />
        {:else}
          <ChevronRight size={14} class="text-gray-400" />
        {/if}
        <span class="text-orange-400">{'{'}</span>
        {#if !rendered.expanded}
          <span class="text-gray-500">...{rendered.keys.length} properties</span>
        {/if}
      </button>

      {#if rendered.expanded}
        <div class="ml-4">
          {#each rendered.keys as key}
            <div class="flex items-start gap-2">
              <span class="text-blue-300">{key}:</span>
              <div class="flex-1">
                {@render structuredDataTree(rendered.value[key], `${path}.${key}`, level + 1)}
              </div>
            </div>
          {/each}
        </div>
        <div style="padding-left: {rendered.indent}px">
          <span class="text-orange-400">{'}'}</span>
        </div>
      {/if}
    </div>
  {:else if rendered.type === 'array'}
    <div style="padding-left: {rendered.indent}px">
      <button
        onclick={() => togglePath(rendered.path)}
        class="flex items-center gap-1 hover:bg-gray-800 dark:hover:bg-gray-900 px-1 rounded"
      >
        {#if rendered.expanded}
          <ChevronDown size={14} class="text-gray-400" />
        {:else}
          <ChevronRight size={14} class="text-gray-400" />
        {/if}
        <span class="text-pink-400">{'['}</span>
        {#if !rendered.expanded}
          <span class="text-gray-500">...{rendered.length} items</span>
        {/if}
      </button>

      {#if rendered.expanded}
        <div class="ml-4">
          {#each rendered.value as item, index}
            <div class="flex items-start gap-2">
              <span class="text-gray-500">[{index}]:</span>
              <div class="flex-1">
                {@render structuredDataTree(item, `${path}[${index}]`, level + 1)}
              </div>
            </div>
          {/each}
        </div>
        <div style="padding-left: {rendered.indent}px">
          <span class="text-pink-400">{']'}</span>
        </div>
      {/if}
    </div>
  {:else}
    <span class="{getTypeColor(rendered.valueType)}">
      {#if rendered.valueType === 'string'}
        "{rendered.value}"
      {:else if rendered.valueType === 'null'}
        null
      {:else}
        {rendered.value}
      {/if}
    </span>
  {/if}
{/snippet}
