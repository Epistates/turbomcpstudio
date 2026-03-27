<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    benchmarkStore,
    type BenchmarkReport,
    type CallRecord,
    type BenchmarkSession,
    type ReportComparison,
    type SuccessRateComparison,
    type ThroughputComparison,
  } from '$lib/stores/benchmarkStore';

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  interface Props {
    proxyId: string;
    proxyName?: string;
  }

  let { proxyId, proxyName = 'Proxy' }: Props = $props();

  // ---------------------------------------------------------------------------
  // Local state
  // ---------------------------------------------------------------------------

  type Tab = 'overview' | 'tools' | 'methods' | 'live' | 'report';
  let activeTab = $state<Tab>('overview');

  let sessionName = $state('');
  let starting = $state(false);
  let stopping = $state(false);
  let loadingReport = $state(false);
  let localError = $state<string | null>(null);

  // Live feed
  let liveFeedEl = $state<HTMLDivElement | null>(null);
  let livePaused = $state(false);
  let liveRecords = $state<CallRecord[]>([]);

  // Report comparison
  let compareWithId = $state<string>('');

  // Duration timer
  let sessionStartTime = $state<Date | null>(null);
  let durationDisplay = $state('0s');
  let durationTimer: ReturnType<typeof setInterval> | null = null;

  // Poll timer for live records
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  // Store subscriptions — seed with correct shape, not the store object itself
  interface LocalStoreState {
    sessions: BenchmarkSession[];
    activeSessionId: string | null;
    currentReport: BenchmarkReport | null;
    liveRecords: CallRecord[];
    comparison: ReportComparison | null;
    savedReports: BenchmarkReport[];
    loading: boolean;
    error: string | null;
  }

  let storeState = $state<LocalStoreState>({
    sessions: [],
    activeSessionId: null,
    currentReport: null,
    liveRecords: [],
    comparison: null,
    savedReports: [],
    loading: false,
    error: null,
  });

  const unsubscribe = benchmarkStore.subscribe((s) => {
    storeState = s;
    // Keep live records in sync
    if (s.liveRecords.length > 0) {
      liveRecords = s.liveRecords;
    }
  });

  // ---------------------------------------------------------------------------
  // Derived values from store state
  // ---------------------------------------------------------------------------

  let activeSessionForProxy = $derived(
    storeState.sessions.find(
      (sess) => sess.active && storeState.activeSessionId === sess.id
    ) ?? null
  );

  let isRunning = $derived(activeSessionForProxy !== null);

  let currentReport = $derived(storeState.currentReport);

  let primaryBackend = $derived(
    currentReport?.backends[0] ?? null
  );

  // ---------------------------------------------------------------------------
  // Formatting helpers
  // ---------------------------------------------------------------------------

  function fmtMs(ms: number): string {
    return ms.toFixed(1);
  }

  function fmtNum(n: number): string {
    return n.toLocaleString('en-US');
  }

  function fmtBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  }

  function fmtSuccessRate(rate: number): string {
    return rate.toFixed(1) + '%';
  }

  function fmtLatencyUs(us: number): string {
    return (us / 1000).toFixed(1) + ' ms';
  }

  function fmtTimestamp(ts: string): string {
    try {
      return new Date(ts).toLocaleTimeString('en-US', {
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      });
    } catch {
      return ts;
    }
  }

  function successRateClass(rate: number): string {
    if (rate >= 95) return 'rate-good';
    if (rate >= 80) return 'rate-warn';
    return 'rate-bad';
  }

  function deltaClass(delta: number): string {
    if (delta < 0) return 'delta-good';
    if (delta > 0) return 'delta-bad';
    return 'delta-neutral';
  }

  function fmtDelta(delta: number, pct: number | null): string {
    const sign = delta >= 0 ? '+' : '';
    if (pct !== null) {
      return `${sign}${pct.toFixed(1)}%`;
    }
    return `${sign}${delta.toFixed(1)} ms`;
  }

  function updateDuration() {
    if (!sessionStartTime) {
      durationDisplay = '0s';
      return;
    }
    const secs = Math.floor((Date.now() - sessionStartTime.getTime()) / 1000);
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    if (h > 0) durationDisplay = `${h}h ${m}m ${s}s`;
    else if (m > 0) durationDisplay = `${m}m ${s}s`;
    else durationDisplay = `${s}s`;
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  async function handleStart() {
    starting = true;
    localError = null;
    try {
      const name = sessionName.trim() || `${proxyName} – ${new Date().toLocaleTimeString()}`;
      const sessionId = await benchmarkStore.startSession(proxyId, name);
      sessionStartTime = new Date();
      durationTimer = setInterval(updateDuration, 1000);
      pollTimer = setInterval(async () => {
        if (!livePaused && sessionId) {
          try {
            await benchmarkStore.getLiveRecords(sessionId);
          } catch {
            // Non-fatal: live polling may fail transiently
          }
        }
      }, 2000);
      // Switch to live tab automatically
      activeTab = 'live';
    } catch (err) {
      localError = err instanceof Error ? err.message : String(err);
    } finally {
      starting = false;
    }
  }

  async function handleStop() {
    const sessionId = storeState.activeSessionId;
    if (!sessionId) return;

    stopping = true;
    localError = null;
    clearTimers();
    sessionStartTime = null;

    try {
      await benchmarkStore.stopSession(sessionId);
      activeTab = 'report';
    } catch (err) {
      localError = err instanceof Error ? err.message : String(err);
    } finally {
      stopping = false;
    }
  }

  async function handleGenerateReport() {
    const sessionId = storeState.activeSessionId;
    if (!sessionId) return;
    loadingReport = true;
    localError = null;
    try {
      await benchmarkStore.getReport(sessionId);
      activeTab = 'report';
    } catch (err) {
      localError = err instanceof Error ? err.message : String(err);
    } finally {
      loadingReport = false;
    }
  }

  async function handleCompare() {
    const reportA = currentReport;
    if (!reportA || !compareWithId) return;
    const reportB = storeState.savedReports.find((r) => r.session_id === compareWithId);
    if (!reportB) return;
    try {
      await benchmarkStore.compareReports(reportA, reportB);
    } catch (err) {
      localError = err instanceof Error ? err.message : String(err);
    }
  }

  function handleExportJson() {
    if (!currentReport) return;
    const blob = new Blob([JSON.stringify(currentReport, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `benchmark-${currentReport.session_id.slice(0, 8)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function scrollLiveFeedToBottom() {
    if (liveFeedEl && !livePaused) {
      liveFeedEl.scrollTop = liveFeedEl.scrollHeight;
    }
  }

  function clearTimers() {
    if (durationTimer) { clearInterval(durationTimer); durationTimer = null; }
    if (pollTimer) { clearInterval(pollTimer); pollTimer = null; }
  }

  // ---------------------------------------------------------------------------
  // Effects
  // ---------------------------------------------------------------------------

  $effect(() => {
    // Auto-scroll live feed when new records arrive
    if (activeTab === 'live') {
      scrollLiveFeedToBottom();
    }
  });

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  onMount(async () => {
    await benchmarkStore.loadSessions();
  });

  onDestroy(() => {
    clearTimers();
    unsubscribe();
  });
</script>

<div class="benchmark-panel">
  <!-- Panel header -->
  <div class="panel-header">
    <div class="header-left">
      <svg class="header-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
        />
      </svg>
      <h3 class="panel-title">Benchmark</h3>
      {#if isRunning}
        <span class="session-badge running">
          <span class="pulse-dot"></span>
          Running — {durationDisplay}
        </span>
      {/if}
    </div>

    <div class="header-right">
      {#if !isRunning}
        <div class="session-name-row">
          <input
            class="session-name-input"
            type="text"
            placeholder="Session name (optional)"
            bind:value={sessionName}
            disabled={starting}
          />
          <button
            class="btn-start"
            onclick={handleStart}
            disabled={starting}
            aria-label="Start benchmark session"
          >
            {#if starting}
              <span class="spinner-sm"></span>
            {:else}
              <svg class="icon-sm" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
                />
              </svg>
            {/if}
            Start
          </button>
        </div>
      {:else}
        <button
          class="btn-stop"
          onclick={handleStop}
          disabled={stopping}
          aria-label="Stop benchmark session"
        >
          {#if stopping}
            <span class="spinner-sm"></span>
          {:else}
            <svg class="icon-sm" fill="currentColor" viewBox="0 0 24 24">
              <rect x="6" y="6" width="12" height="12" rx="1" />
            </svg>
          {/if}
          Stop
        </button>
        <button
          class="btn-ghost"
          onclick={handleGenerateReport}
          disabled={loadingReport}
          aria-label="Generate report snapshot"
        >
          {#if loadingReport}
            <span class="spinner-sm"></span>
          {:else}
            <svg class="icon-sm" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
          {/if}
          Snapshot
        </button>
      {/if}
    </div>
  </div>

  <!-- Error display -->
  {#if localError}
    <div class="error-bar" role="alert">
      <svg class="icon-sm error-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"
        />
      </svg>
      <span>{localError}</span>
      <button class="error-dismiss" onclick={() => (localError = null)} aria-label="Dismiss error">
        ✕
      </button>
    </div>
  {/if}

  <!-- Tab bar -->
  <div class="tab-bar" role="tablist" aria-label="Benchmark tabs">
    {#each [
      { id: 'overview', label: 'Overview' },
      { id: 'tools', label: 'Per-Tool' },
      { id: 'methods', label: 'Per-Method' },
      { id: 'live', label: 'Live Feed' },
      { id: 'report', label: 'Report' },
    ] as tab}
      <button
        class="tab-btn"
        class:active={activeTab === tab.id}
        role="tab"
        aria-selected={activeTab === tab.id}
        onclick={() => (activeTab = tab.id as Tab)}
      >
        {tab.label}
        {#if tab.id === 'live' && isRunning}
          <span class="live-dot"></span>
        {/if}
      </button>
    {/each}
  </div>

  <!-- Tab content -->
  <div class="tab-content">

    <!-- ------------------------------------------------------------------ -->
    <!-- OVERVIEW TAB                                                        -->
    <!-- ------------------------------------------------------------------ -->
    {#if activeTab === 'overview'}
      {#if primaryBackend}
        <div class="overview-grid">
          <div class="stat-card">
            <div class="stat-label">Total Requests</div>
            <div class="stat-value">{fmtNum(primaryBackend.total_calls)}</div>
            <div class="stat-sub">{fmtNum(primaryBackend.total_tool_calls)} tool calls</div>
          </div>

          <div class="stat-card">
            <div class="stat-label">Success Rate</div>
            <div class="stat-value {successRateClass(primaryBackend.success_rate)}">
              {fmtSuccessRate(primaryBackend.success_rate)}
            </div>
            <div class="stat-sub">
              {fmtNum(primaryBackend.failure_count)} failures
            </div>
          </div>

          <div class="stat-card">
            <div class="stat-label">Avg Latency</div>
            <div class="stat-value">
              {primaryBackend.overall_latency
                ? fmtMs(primaryBackend.overall_latency.mean_ms) + ' ms'
                : '—'}
            </div>
            <div class="stat-sub">
              {#if primaryBackend.overall_latency}
                P95: {fmtMs(primaryBackend.overall_latency.p95_ms)} ms
              {:else}
                No data
              {/if}
            </div>
          </div>

          <div class="stat-card">
            <div class="stat-label">Est. Tokens</div>
            <div class="stat-value">{fmtNum(primaryBackend.estimated_total_tokens)}</div>
            <div class="stat-sub">
              {fmtNum(primaryBackend.estimated_input_tokens)} in /
              {fmtNum(primaryBackend.estimated_output_tokens)} out
            </div>
          </div>
        </div>

        {#if primaryBackend.overall_latency}
          <div class="latency-section">
            <h4 class="section-heading">Latency Distribution</h4>
            <div class="latency-grid">
              {#each [
                { label: 'Min', value: primaryBackend.overall_latency.min_ms },
                { label: 'P50', value: primaryBackend.overall_latency.p50_ms },
                { label: 'P95', value: primaryBackend.overall_latency.p95_ms },
                { label: 'P99', value: primaryBackend.overall_latency.p99_ms },
                { label: 'Max', value: primaryBackend.overall_latency.max_ms },
                { label: 'Std Dev', value: primaryBackend.overall_latency.std_dev_ms },
              ] as stat}
                <div class="latency-cell">
                  <span class="latency-label">{stat.label}</span>
                  <span class="latency-value">{fmtMs(stat.value)} ms</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <div class="throughput-section">
          <h4 class="section-heading">Throughput</h4>
          <div class="throughput-grid">
            <div class="throughput-cell">
              <span class="throughput-label">Bytes In</span>
              <span class="throughput-value">{fmtBytes(primaryBackend.total_input_bytes)}</span>
            </div>
            <div class="throughput-cell">
              <span class="throughput-label">Bytes Out</span>
              <span class="throughput-value">{fmtBytes(primaryBackend.total_output_bytes)}</span>
            </div>
            <div class="throughput-cell">
              <span class="throughput-label">Total</span>
              <span class="throughput-value">{fmtBytes(primaryBackend.total_bytes)}</span>
            </div>
            {#if currentReport && currentReport.duration_secs > 0}
              <div class="throughput-cell">
                <span class="throughput-label">Duration</span>
                <span class="throughput-value">{currentReport.duration_secs.toFixed(1)} s</span>
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="empty-state">
          <svg class="empty-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
            />
          </svg>
          <p class="empty-title">No benchmark data</p>
          <p class="empty-desc">Start a session above, then send some MCP requests through this proxy.</p>
        </div>
      {/if}

    <!-- ------------------------------------------------------------------ -->
    <!-- PER-TOOL TAB                                                        -->
    <!-- ------------------------------------------------------------------ -->
    {:else if activeTab === 'tools'}
      {#if primaryBackend && primaryBackend.tools.length > 0}
        <div class="table-container">
          <table class="data-table">
            <thead>
              <tr>
                <th class="col-name">Tool</th>
                <th class="col-num">Calls</th>
                <th class="col-rate">OK%</th>
                <th class="col-num">Tokens In</th>
                <th class="col-num">Tokens Out</th>
                <th class="col-ms">P50 ms</th>
                <th class="col-ms">P95 ms</th>
                <th class="col-ms">P99 ms</th>
              </tr>
            </thead>
            <tbody>
              {#each primaryBackend.tools as tool (tool.name)}
                {@const successPct = tool.call_count > 0
                  ? (tool.success_count / tool.call_count) * 100
                  : 0}
                <tr>
                  <td class="cell-name">
                    <code>{tool.name}</code>
                  </td>
                  <td class="cell-num">{fmtNum(tool.call_count)}</td>
                  <td class="cell-rate">
                    <span class="rate-pill {successRateClass(successPct)}">
                      {successPct.toFixed(0)}%
                    </span>
                  </td>
                  <td class="cell-num">~{fmtNum(tool.estimated_input_tokens)}</td>
                  <td class="cell-num">~{fmtNum(tool.estimated_output_tokens)}</td>
                  <td class="cell-ms">
                    {tool.latency ? fmtMs(tool.latency.p50_ms) : '—'}
                  </td>
                  <td class="cell-ms">
                    {tool.latency ? fmtMs(tool.latency.p95_ms) : '—'}
                  </td>
                  <td class="cell-ms">
                    {tool.latency ? fmtMs(tool.latency.p99_ms) : '—'}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="empty-state">
          <p class="empty-title">No tool data yet</p>
          <p class="empty-desc">Tool call records will appear here once the proxy receives requests.</p>
        </div>
      {/if}

    <!-- ------------------------------------------------------------------ -->
    <!-- PER-METHOD TAB                                                      -->
    <!-- ------------------------------------------------------------------ -->
    {:else if activeTab === 'methods'}
      {#if primaryBackend && primaryBackend.methods.length > 0}
        <div class="table-container">
          <table class="data-table">
            <thead>
              <tr>
                <th class="col-name">Method</th>
                <th class="col-num">Calls</th>
                <th class="col-bytes">Bytes In</th>
                <th class="col-bytes">Bytes Out</th>
                <th class="col-num">Tokens In</th>
                <th class="col-num">Tokens Out</th>
                <th class="col-ms">Mean ms</th>
                <th class="col-ms">P95 ms</th>
              </tr>
            </thead>
            <tbody>
              {#each primaryBackend.methods as method (method.method)}
                <tr>
                  <td class="cell-name">
                    <code>{method.method}</code>
                  </td>
                  <td class="cell-num">{fmtNum(method.call_count)}</td>
                  <td class="cell-bytes">{fmtBytes(method.total_input_bytes)}</td>
                  <td class="cell-bytes">{fmtBytes(method.total_output_bytes)}</td>
                  <td class="cell-num">~{fmtNum(method.estimated_input_tokens)}</td>
                  <td class="cell-num">~{fmtNum(method.estimated_output_tokens)}</td>
                  <td class="cell-ms">
                    {method.latency ? fmtMs(method.latency.mean_ms) : '—'}
                  </td>
                  <td class="cell-ms">
                    {method.latency ? fmtMs(method.latency.p95_ms) : '—'}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="empty-state">
          <p class="empty-title">No method data yet</p>
          <p class="empty-desc">RPC method summaries will appear here once the proxy handles requests.</p>
        </div>
      {/if}

    <!-- ------------------------------------------------------------------ -->
    <!-- LIVE FEED TAB                                                       -->
    <!-- ------------------------------------------------------------------ -->
    {:else if activeTab === 'live'}
      <div class="live-toolbar">
        <span class="live-count">{liveRecords.length} records (last 100)</span>
        <button
          class="btn-pause"
          class:paused={livePaused}
          onclick={() => (livePaused = !livePaused)}
          aria-label={livePaused ? 'Resume auto-scroll' : 'Pause auto-scroll'}
        >
          {#if livePaused}
            <svg class="icon-xs" fill="currentColor" viewBox="0 0 24 24">
              <path d="M8 5v14l11-7z" />
            </svg>
            Resume
          {:else}
            <svg class="icon-xs" fill="currentColor" viewBox="0 0 24 24">
              <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
            </svg>
            Pause
          {/if}
        </button>
      </div>

      <div class="live-feed" bind:this={liveFeedEl}>
        {#if liveRecords.length === 0}
          <div class="live-empty">
            {#if isRunning}
              <span class="pulse-dot"></span> Waiting for requests…
            {:else}
              Start a session to see live call records.
            {/if}
          </div>
        {:else}
          {#each liveRecords as rec, i (i)}
            <div class="record-row" class:record-error={!rec.success}>
              <span class="record-time">{fmtTimestamp(rec.timestamp)}</span>
              <span class="record-method">{rec.method}</span>
              {#if rec.tool_name}
                <span class="record-tool">{rec.tool_name}</span>
              {/if}
              <span class="record-latency">{fmtLatencyUs(rec.latency_us)}</span>
              <span class="record-bytes">{fmtBytes(rec.request_bytes + rec.response_bytes)}</span>
              {#if rec.success}
                <span class="record-badge ok">OK</span>
              {:else}
                <span class="record-badge err" title={rec.error_message ?? ''}>ERR</span>
              {/if}
            </div>
          {/each}
        {/if}
      </div>

    <!-- ------------------------------------------------------------------ -->
    <!-- REPORT TAB                                                          -->
    <!-- ------------------------------------------------------------------ -->
    {:else if activeTab === 'report'}
      {#if currentReport}
        <div class="report-toolbar">
          <div class="report-meta">
            <span class="report-id">Session: {currentReport.session_id.slice(0, 8)}…</span>
            <span class="report-duration">{currentReport.duration_secs.toFixed(1)} s</span>
            <span class="report-records">{fmtNum(currentReport.records.length)} records</span>
          </div>

          <div class="report-actions">
            <button class="btn-ghost" onclick={handleExportJson} aria-label="Export report as JSON">
              <svg class="icon-sm" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                />
              </svg>
              Export JSON
            </button>

            {#if storeState.savedReports.length > 1}
              <div class="compare-row">
                <select
                  class="compare-select"
                  bind:value={compareWithId}
                  aria-label="Select report to compare with"
                >
                  <option value="">Compare with…</option>
                  {#each storeState.savedReports.filter(
                    (r: BenchmarkReport) => r.session_id !== currentReport.session_id
                  ) as report}
                    <option value={report.session_id}>
                      {report.session_id.slice(0, 8)} — {report.duration_secs.toFixed(0)}s
                    </option>
                  {/each}
                </select>
                <button
                  class="btn-ghost"
                  onclick={handleCompare}
                  disabled={!compareWithId}
                  aria-label="Run comparison"
                >
                  Compare
                </button>
              </div>
            {/if}
          </div>
        </div>

        <!-- Comparison results -->
        {#if storeState.comparison}
          {@const cmp = storeState.comparison}
          <div class="comparison-section">
            <div class="comparison-header">
              <h4 class="section-heading">Comparison</h4>
              <button
                class="btn-ghost btn-xs"
                onclick={() => benchmarkStore.clearComparison()}
                aria-label="Clear comparison"
              >
                Clear
              </button>
            </div>

            {#each cmp.latency as backend_lat (backend_lat.backend_name)}
              <div class="comparison-block">
                <div class="comparison-block-title">{backend_lat.backend_name}</div>
                <div class="comparison-grid">
                  {#each [
                    { label: 'Mean', delta: backend_lat.mean },
                    { label: 'P50', delta: backend_lat.p50 },
                    { label: 'P95', delta: backend_lat.p95 },
                    { label: 'P99', delta: backend_lat.p99 },
                  ] as item}
                    <div class="comparison-cell">
                      <span class="comparison-label">{item.label}</span>
                      <span class="comparison-delta {deltaClass(item.delta.delta_ms)}">
                        {fmtDelta(item.delta.delta_ms, item.delta.delta_pct)}
                      </span>
                    </div>
                  {/each}
                </div>

                {#each cmp.success_rates.filter(
                  (r: SuccessRateComparison) => r.backend_name === backend_lat.backend_name
                ) as sr}
                  <div class="comparison-row">
                    <span class="comparison-label">Success Rate</span>
                    <span class="comparison-value">
                      {sr.success_rate_a.toFixed(1)}% → {sr.success_rate_b.toFixed(1)}%
                    </span>
                    <span class="comparison-delta {deltaClass(-sr.delta_pct_points)}">
                      {sr.delta_pct_points >= 0 ? '+' : ''}{sr.delta_pct_points.toFixed(1)} pp
                    </span>
                  </div>
                {/each}

                {#each cmp.throughput.filter(
                  (t: ThroughputComparison) => t.backend_name === backend_lat.backend_name
                ) as tp}
                  <div class="comparison-row">
                    <span class="comparison-label">Throughput</span>
                    <span class="comparison-value">
                      {fmtBytes(Math.round(tp.bytes_per_sec_a))}/s →
                      {fmtBytes(Math.round(tp.bytes_per_sec_b))}/s
                    </span>
                    {#if tp.delta_pct !== null}
                      <span class="comparison-delta {deltaClass(-tp.delta_pct)}">
                        {tp.delta_pct >= 0 ? '+' : ''}{tp.delta_pct.toFixed(1)}%
                      </span>
                    {/if}
                  </div>
                {/each}
              </div>
            {/each}
          </div>
        {/if}

        <!-- Per-backend summary cards -->
        {#each currentReport.backends as backend (backend.name)}
          <div class="report-backend-card">
            <div class="backend-card-header">
              <span class="backend-name">{backend.name}</span>
              <span class="backend-rate {successRateClass(backend.success_rate)}">
                {fmtSuccessRate(backend.success_rate)} success
              </span>
            </div>

            <div class="backend-stats">
              <div class="bstat">
                <span class="bstat-label">Calls</span>
                <span class="bstat-value">{fmtNum(backend.total_calls)}</span>
              </div>
              <div class="bstat">
                <span class="bstat-label">Tokens</span>
                <span class="bstat-value">~{fmtNum(backend.estimated_total_tokens)}</span>
              </div>
              <div class="bstat">
                <span class="bstat-label">Bytes</span>
                <span class="bstat-value">{fmtBytes(backend.total_bytes)}</span>
              </div>
              {#if backend.overall_latency}
                <div class="bstat">
                  <span class="bstat-label">P50</span>
                  <span class="bstat-value">{fmtMs(backend.overall_latency.p50_ms)} ms</span>
                </div>
                <div class="bstat">
                  <span class="bstat-label">P99</span>
                  <span class="bstat-value">{fmtMs(backend.overall_latency.p99_ms)} ms</span>
                </div>
              {/if}
            </div>
          </div>
        {/each}

      {:else}
        <div class="empty-state">
          <svg class="empty-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <p class="empty-title">No report yet</p>
          <p class="empty-desc">
            Stop a session or click "Snapshot" to generate a benchmark report.
          </p>
        </div>
      {/if}
    {/if}

  </div>
</div>

<style>
  /* =========================================================================
   * Layout
   * ========================================================================= */

  .benchmark-panel {
    display: flex;
    flex-direction: column;
    background: var(--mcp-surface-secondary, var(--color-bg-secondary));
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-lg);
    overflow: hidden;
  }

  /* =========================================================================
   * Panel Header
   * ========================================================================= */

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--mcp-space-4);
    padding: var(--mcp-space-3) var(--mcp-space-4);
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
    background: var(--mcp-surface-primary, var(--color-bg));
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .header-icon {
    width: 18px;
    height: 18px;
    color: var(--mcp-primary-500, var(--color-primary));
    flex-shrink: 0;
  }

  .panel-title {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary, var(--color-text));
    margin: 0;
  }

  .session-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--mcp-space-1-5);
    padding: var(--mcp-space-0-5) var(--mcp-space-2);
    border-radius: var(--mcp-radius-full);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-medium);
  }

  .session-badge.running {
    background: var(--mcp-success-50);
    color: var(--mcp-success-700, #15803d);
    border: 1px solid var(--mcp-success-500);
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--mcp-success-500);
    animation: pulseBeat 1.5s ease-in-out infinite;
    flex-shrink: 0;
  }

  @keyframes pulseBeat {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.5; transform: scale(0.8); }
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .session-name-row {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .session-name-input {
    height: 32px;
    padding: 0 var(--mcp-space-3);
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
    background: var(--mcp-input-background, var(--color-bg));
    color: var(--mcp-text-primary, var(--color-text));
    font-size: var(--mcp-text-xs);
    font-family: inherit;
    min-width: 160px;
    outline: none;
    transition: border-color var(--mcp-transition-fast);
  }

  .session-name-input:focus {
    border-color: var(--mcp-border-focus, var(--color-primary));
    box-shadow: var(--mcp-input-focus-ring);
  }

  .session-name-input::placeholder {
    color: var(--mcp-text-disabled);
  }

  .btn-start,
  .btn-stop,
  .btn-ghost {
    display: inline-flex;
    align-items: center;
    gap: var(--mcp-space-1-5);
    height: 32px;
    padding: 0 var(--mcp-space-3);
    border-radius: var(--mcp-radius-md);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-medium);
    font-family: inherit;
    cursor: pointer;
    transition: all var(--mcp-transition-fast);
    white-space: nowrap;
  }

  .btn-start {
    background: var(--mcp-success-500);
    color: #fff;
    border: 1px solid var(--mcp-success-600);
  }

  .btn-start:hover:not(:disabled) {
    background: var(--mcp-success-600);
  }

  .btn-start:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-stop {
    background: var(--mcp-error-500);
    color: #fff;
    border: 1px solid var(--mcp-error-600);
  }

  .btn-stop:hover:not(:disabled) {
    background: var(--mcp-error-600);
  }

  .btn-stop:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-ghost {
    background: transparent;
    color: var(--mcp-text-secondary, var(--color-text-secondary));
    border: 1px solid var(--mcp-border-primary, var(--color-border));
  }

  .btn-ghost:hover:not(:disabled) {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-primary, var(--color-text));
    border-color: var(--mcp-border-secondary);
  }

  .btn-ghost:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-xs {
    height: 24px;
    padding: 0 var(--mcp-space-2);
    font-size: var(--mcp-text-xs);
  }

  .icon-sm {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  .icon-xs {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }

  .spinner-sm {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* =========================================================================
   * Error bar
   * ========================================================================= */

  .error-bar {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-2) var(--mcp-space-4);
    background: var(--mcp-error-50);
    border-bottom: 1px solid var(--mcp-border-error);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-error-700, #b91c1c);
  }

  .error-icon {
    flex-shrink: 0;
    color: var(--mcp-error-500);
  }

  .error-bar span {
    flex: 1;
  }

  .error-dismiss {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--mcp-error-500);
    font-size: var(--mcp-text-sm);
    padding: 0;
    line-height: 1;
  }

  /* =========================================================================
   * Tab bar
   * ========================================================================= */

  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
    background: var(--mcp-surface-primary, var(--color-bg));
    overflow-x: auto;
    scrollbar-width: none;
  }

  .tab-bar::-webkit-scrollbar {
    display: none;
  }

  .tab-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--mcp-space-1-5);
    padding: var(--mcp-space-2) var(--mcp-space-4);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-tertiary, var(--color-text-secondary));
    white-space: nowrap;
    transition: all var(--mcp-transition-fast);
    font-family: inherit;
    margin-bottom: -1px;
  }

  .tab-btn:hover {
    color: var(--mcp-text-primary, var(--color-text));
    background: var(--mcp-surface-secondary);
  }

  .tab-btn.active {
    color: var(--mcp-primary-500, var(--color-primary));
    border-bottom-color: var(--mcp-primary-500, var(--color-primary));
    font-weight: var(--mcp-font-semibold);
  }

  .live-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--mcp-success-500);
    animation: pulseBeat 1.5s ease-in-out infinite;
  }

  /* =========================================================================
   * Tab content area
   * ========================================================================= */

  .tab-content {
    flex: 1;
    overflow: auto;
    padding: var(--mcp-space-4);
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-4);
  }

  /* =========================================================================
   * Overview tab
   * ========================================================================= */

  .overview-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: var(--mcp-space-3);
  }

  .stat-card {
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-primary, var(--color-bg));
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
    display: flex;
    flex-direction: column;
    gap: var(--mcp-space-1);
  }

  .stat-label {
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-semibold);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--mcp-text-tertiary, var(--color-text-secondary));
  }

  .stat-value {
    font-size: var(--mcp-text-2xl);
    font-weight: var(--mcp-font-bold);
    color: var(--mcp-primary-500, var(--color-primary));
    font-family: var(--mcp-font-mono);
    line-height: 1.1;
  }

  .stat-sub {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
  }

  .section-heading {
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-semibold);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--mcp-text-secondary, var(--color-text-secondary));
    margin: 0 0 var(--mcp-space-2) 0;
  }

  .latency-section,
  .throughput-section {
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-primary, var(--color-bg));
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
  }

  .latency-grid,
  .throughput-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
    gap: var(--mcp-space-2);
  }

  .latency-cell,
  .throughput-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .latency-label,
  .throughput-label {
    font-size: 10px;
    font-weight: var(--mcp-font-semibold);
    text-transform: uppercase;
    color: var(--mcp-text-disabled);
    letter-spacing: 0.3px;
  }

  .latency-value,
  .throughput-value {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary, var(--color-text));
    font-family: var(--mcp-font-mono);
  }

  /* =========================================================================
   * Success rate colors
   * ========================================================================= */

  .rate-good { color: var(--mcp-success-600); }
  .rate-warn { color: var(--mcp-warning-600); }
  .rate-bad  { color: var(--mcp-error-500); }

  /* =========================================================================
   * Data tables (tools / methods)
   * ========================================================================= */

  .table-container {
    overflow-x: auto;
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--mcp-text-xs);
  }

  .data-table thead {
    background: var(--mcp-surface-tertiary, var(--color-bg-secondary));
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .data-table th {
    padding: var(--mcp-space-2) var(--mcp-space-3);
    text-align: left;
    font-weight: var(--mcp-font-semibold);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    font-size: 10px;
    color: var(--mcp-text-tertiary, var(--color-text-secondary));
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
    white-space: nowrap;
  }

  .data-table th.col-num,
  .data-table th.col-ms,
  .data-table th.col-rate,
  .data-table th.col-bytes {
    text-align: right;
  }

  .data-table tbody tr {
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
    transition: background var(--mcp-transition-fast);
  }

  .data-table tbody tr:last-child {
    border-bottom: none;
  }

  .data-table tbody tr:hover {
    background: var(--mcp-surface-secondary, var(--color-bg-secondary));
  }

  .data-table td {
    padding: var(--mcp-space-2) var(--mcp-space-3);
    color: var(--mcp-text-primary, var(--color-text));
    vertical-align: middle;
  }

  .cell-name code {
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-primary-500, var(--color-primary));
  }

  .cell-num,
  .cell-ms,
  .cell-rate,
  .cell-bytes {
    text-align: right;
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-secondary, var(--color-text-secondary));
  }

  .rate-pill {
    display: inline-block;
    padding: 1px var(--mcp-space-2);
    border-radius: var(--mcp-radius-full);
    font-size: 10px;
    font-weight: var(--mcp-font-semibold);
  }

  .rate-pill.rate-good {
    background: var(--mcp-success-50);
    color: var(--mcp-success-700, #15803d);
    border: 1px solid var(--mcp-success-500);
  }

  .rate-pill.rate-warn {
    background: var(--mcp-warning-50);
    color: var(--mcp-warning-700, #b45309);
    border: 1px solid var(--mcp-warning-500);
  }

  .rate-pill.rate-bad {
    background: var(--mcp-error-50);
    color: var(--mcp-error-700, #b91c1c);
    border: 1px solid var(--mcp-error-500);
  }

  /* =========================================================================
   * Live feed tab
   * ========================================================================= */

  .live-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: var(--mcp-space-2);
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
  }

  .live-count {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
  }

  .btn-pause {
    display: inline-flex;
    align-items: center;
    gap: var(--mcp-space-1);
    height: 26px;
    padding: 0 var(--mcp-space-2);
    background: transparent;
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-medium);
    color: var(--mcp-text-secondary);
    cursor: pointer;
    font-family: inherit;
    transition: all var(--mcp-transition-fast);
  }

  .btn-pause:hover {
    background: var(--mcp-surface-tertiary);
    color: var(--mcp-text-primary);
  }

  .btn-pause.paused {
    background: var(--mcp-warning-50);
    border-color: var(--mcp-warning-500);
    color: var(--mcp-warning-700, #b45309);
  }

  .live-feed {
    flex: 1;
    overflow-y: auto;
    max-height: 340px;
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
    background: var(--mcp-surface-primary, var(--color-bg));
    font-family: var(--mcp-font-mono);
    font-size: var(--mcp-text-xs);
    scrollbar-width: thin;
  }

  .live-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-6);
    color: var(--mcp-text-tertiary);
    font-family: var(--mcp-font-sans);
  }

  .record-row {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-1-5) var(--mcp-space-3);
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
    transition: background var(--mcp-transition-fast);
  }

  .record-row:last-child {
    border-bottom: none;
  }

  .record-row:hover {
    background: var(--mcp-surface-secondary);
  }

  .record-row.record-error {
    background: var(--mcp-error-50);
  }

  .record-time {
    color: var(--mcp-text-disabled);
    flex-shrink: 0;
    font-size: 10px;
  }

  .record-method {
    color: var(--mcp-primary-500, var(--color-primary));
    flex-shrink: 0;
    min-width: 90px;
  }

  .record-tool {
    color: var(--mcp-text-secondary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .record-latency {
    color: var(--mcp-warning-600);
    flex-shrink: 0;
    min-width: 60px;
    text-align: right;
  }

  .record-bytes {
    color: var(--mcp-text-disabled);
    flex-shrink: 0;
    min-width: 60px;
    text-align: right;
  }

  .record-badge {
    display: inline-block;
    padding: 1px 6px;
    border-radius: var(--mcp-radius-base);
    font-size: 9px;
    font-weight: var(--mcp-font-bold);
    letter-spacing: 0.4px;
    flex-shrink: 0;
  }

  .record-badge.ok {
    background: var(--mcp-success-50);
    color: var(--mcp-success-700, #15803d);
    border: 1px solid var(--mcp-success-500);
  }

  .record-badge.err {
    background: var(--mcp-error-50);
    color: var(--mcp-error-700, #b91c1c);
    border: 1px solid var(--mcp-error-500);
  }

  /* =========================================================================
   * Report tab
   * ========================================================================= */

  .report-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--mcp-space-3);
    flex-wrap: wrap;
    padding: var(--mcp-space-3);
    background: var(--mcp-surface-primary, var(--color-bg));
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
  }

  .report-meta {
    display: flex;
    gap: var(--mcp-space-3);
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    font-family: var(--mcp-font-mono);
  }

  .report-id { color: var(--mcp-primary-500); }

  .report-actions {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
    flex-wrap: wrap;
  }

  .compare-row {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-2);
  }

  .compare-select {
    height: 32px;
    padding: 0 var(--mcp-space-2);
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
    background: var(--mcp-input-background, var(--color-bg));
    color: var(--mcp-text-primary, var(--color-text));
    font-size: var(--mcp-text-xs);
    font-family: inherit;
    cursor: pointer;
    outline: none;
  }

  /* Comparison section */

  .comparison-section {
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
    overflow: hidden;
  }

  .comparison-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-tertiary, var(--color-bg-secondary));
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
  }

  .comparison-block {
    padding: var(--mcp-space-3);
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
  }

  .comparison-block:last-child {
    border-bottom: none;
  }

  .comparison-block-title {
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-secondary);
    margin-bottom: var(--mcp-space-2);
  }

  .comparison-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--mcp-space-2);
    margin-bottom: var(--mcp-space-2);
  }

  .comparison-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--mcp-space-2);
    background: var(--mcp-surface-secondary);
    border-radius: var(--mcp-radius-base);
  }

  .comparison-label {
    font-size: 10px;
    font-weight: var(--mcp-font-semibold);
    text-transform: uppercase;
    color: var(--mcp-text-disabled);
  }

  .comparison-delta {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-bold);
    font-family: var(--mcp-font-mono);
  }

  .comparison-row {
    display: flex;
    align-items: center;
    gap: var(--mcp-space-3);
    font-size: var(--mcp-text-xs);
    padding: var(--mcp-space-1) 0;
  }

  .comparison-value {
    flex: 1;
    color: var(--mcp-text-secondary);
    font-family: var(--mcp-font-mono);
  }

  /* Delta colours: lower latency = good, higher = bad */
  .delta-good    { color: var(--mcp-success-500); }
  .delta-bad     { color: var(--mcp-error-500); }
  .delta-neutral { color: var(--mcp-text-tertiary); }

  /* Backend summary cards in report */

  .report-backend-card {
    border: 1px solid var(--mcp-border-primary, var(--color-border));
    border-radius: var(--mcp-radius-md);
    overflow: hidden;
  }

  .backend-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--mcp-space-2) var(--mcp-space-3);
    background: var(--mcp-surface-tertiary, var(--color-bg-secondary));
    border-bottom: 1px solid var(--mcp-border-primary, var(--color-border));
  }

  .backend-name {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary, var(--color-text));
    font-family: var(--mcp-font-mono);
  }

  .backend-rate {
    font-size: var(--mcp-text-xs);
    font-weight: var(--mcp-font-medium);
  }

  .backend-stats {
    display: flex;
    flex-wrap: wrap;
    gap: var(--mcp-space-3);
    padding: var(--mcp-space-3);
  }

  .bstat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 80px;
  }

  .bstat-label {
    font-size: 10px;
    font-weight: var(--mcp-font-semibold);
    text-transform: uppercase;
    color: var(--mcp-text-disabled);
    letter-spacing: 0.3px;
  }

  .bstat-value {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-primary, var(--color-text));
    font-family: var(--mcp-font-mono);
  }

  /* =========================================================================
   * Empty state
   * ========================================================================= */

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--mcp-space-2);
    padding: var(--mcp-space-8) var(--mcp-space-4);
    text-align: center;
  }

  .empty-icon {
    width: 40px;
    height: 40px;
    opacity: 0.3;
    color: var(--mcp-text-tertiary);
  }

  .empty-title {
    font-size: var(--mcp-text-sm);
    font-weight: var(--mcp-font-semibold);
    color: var(--mcp-text-secondary, var(--color-text-secondary));
    margin: 0;
  }

  .empty-desc {
    font-size: var(--mcp-text-xs);
    color: var(--mcp-text-tertiary);
    max-width: 320px;
    margin: 0;
    line-height: var(--mcp-leading-relaxed);
  }

  /* =========================================================================
   * Dark theme overrides
   * ========================================================================= */

  :global([data-theme="dark"]) .session-badge.running {
    background: rgba(16, 185, 129, 0.15);
    color: var(--mcp-success-400, #4ade80);
  }

  :global([data-theme="dark"]) .rate-pill.rate-good {
    background: rgba(16, 185, 129, 0.15);
    color: var(--mcp-success-400, #4ade80);
    border-color: var(--mcp-success-600);
  }

  :global([data-theme="dark"]) .rate-pill.rate-warn {
    background: rgba(245, 158, 11, 0.15);
    color: var(--mcp-warning-400, #fbbf24);
    border-color: var(--mcp-warning-600);
  }

  :global([data-theme="dark"]) .rate-pill.rate-bad {
    background: rgba(239, 68, 68, 0.15);
    color: var(--mcp-error-400, #f87171);
    border-color: var(--mcp-error-600);
  }

  :global([data-theme="dark"]) .record-row.record-error {
    background: rgba(239, 68, 68, 0.1);
  }

  :global([data-theme="dark"]) .record-badge.ok {
    background: rgba(16, 185, 129, 0.15);
    color: var(--mcp-success-400, #4ade80);
    border-color: var(--mcp-success-600);
  }

  :global([data-theme="dark"]) .record-badge.err {
    background: rgba(239, 68, 68, 0.15);
    color: var(--mcp-error-400, #f87171);
    border-color: var(--mcp-error-600);
  }

  :global([data-theme="dark"]) .error-bar {
    background: rgba(239, 68, 68, 0.12);
    color: var(--mcp-error-300, #fca5a5);
  }

  :global([data-theme="dark"]) .help-section {
    background: var(--mcp-surface-accent);
  }
</style>
