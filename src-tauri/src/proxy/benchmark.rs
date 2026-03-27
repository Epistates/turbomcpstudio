//! Benchmark module for MCP proxy performance measurement
//!
//! Tracks per-call latency, throughput, and token estimates across one or more
//! benchmark sessions. Adapted from the turbobench CLI project for the Tauri
//! desktop context where session state lives in-process.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Maximum number of records stored per session before dropping starts.
const MAX_RECORDS: usize = 500_000;

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

/// Estimate token count from raw byte length using the ~4 bytes/token heuristic.
pub fn estimate_tokens(bytes: usize) -> usize {
    (bytes + 3) / 4
}

// ---------------------------------------------------------------------------
// Core record type
// ---------------------------------------------------------------------------

/// A single recorded call through the proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRecord {
    /// Name of the backend this call was sent to.
    pub backend: String,
    /// MCP method (e.g. `"tools/call"`, `"resources/read"`).
    pub method: String,
    /// Tool name, if this was a `tools/call` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// Resource URI, if this was a `resources/read` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_uri: Option<String>,
    /// Prompt name, if this was a `prompts/get` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_name: Option<String>,
    /// End-to-end latency in microseconds.
    pub latency_us: u64,
    /// Request payload size in bytes.
    pub request_bytes: usize,
    /// Response payload size in bytes.
    pub response_bytes: usize,
    /// Estimated input token count (request_bytes / 4).
    pub estimated_input_tokens: usize,
    /// Estimated output token count (response_bytes / 4).
    pub estimated_output_tokens: usize,
    /// Whether the call succeeded.
    pub success: bool,
    /// Error message, if the call failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Wall-clock timestamp when the call was recorded.
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Latency percentile statistics.
///
/// Uses **sample** standard deviation (Bessel's correction, N-1 denominator).
/// Percentiles use linear interpolation between adjacent ranks (NumPy default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub count: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub std_dev_ms: f64,
}

impl LatencyStats {
    /// Compute statistics from a mutable slice of latencies in microseconds.
    ///
    /// Returns `None` if the slice is empty. The slice is sorted in place.
    pub fn from_latencies_us(latencies: &mut [u64]) -> Option<Self> {
        if latencies.is_empty() {
            return None;
        }
        latencies.sort_unstable();
        let n = latencies.len();
        let sum: u64 = latencies.iter().sum();
        let mean = sum as f64 / n as f64;

        // Sample variance (Bessel's correction: divide by N-1 for N > 1).
        let variance = if n > 1 {
            latencies
                .iter()
                .map(|&x| {
                    let d = x as f64 - mean;
                    d * d
                })
                .sum::<f64>()
                / (n - 1) as f64
        } else {
            0.0
        };

        Some(Self {
            count: n,
            min_ms: latencies[0] as f64 / 1_000.0,
            max_ms: latencies[n - 1] as f64 / 1_000.0,
            mean_ms: mean / 1_000.0,
            p50_ms: interpolated_percentile(latencies, 50.0) / 1_000.0,
            p95_ms: interpolated_percentile(latencies, 95.0) / 1_000.0,
            p99_ms: interpolated_percentile(latencies, 99.0) / 1_000.0,
            std_dev_ms: variance.sqrt() / 1_000.0,
        })
    }
}

/// Linear interpolation percentile — same algorithm as NumPy's default method.
fn interpolated_percentile(sorted: &[u64], p: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    if sorted.len() == 1 {
        return sorted[0] as f64;
    }
    let rank = p / 100.0 * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = rank - lo as f64;
    sorted[lo] as f64 * (1.0 - frac) + sorted[hi] as f64 * frac
}

// ---------------------------------------------------------------------------
// Summary types
// ---------------------------------------------------------------------------

/// Per-tool performance summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSummary {
    pub name: String,
    pub call_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_input_bytes: usize,
    pub total_output_bytes: usize,
    pub estimated_input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub latency: Option<LatencyStats>,
}

/// Per-method performance summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodSummary {
    pub method: String,
    pub call_count: usize,
    pub total_input_bytes: usize,
    pub total_output_bytes: usize,
    pub estimated_input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub latency: Option<LatencyStats>,
}

/// Aggregate performance summary for a single backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSummary {
    pub name: String,
    pub total_calls: usize,
    pub total_tool_calls: usize,
    pub success_count: usize,
    pub failure_count: usize,
    /// Success rate as a percentage (0–100).
    pub success_rate: f64,
    pub total_input_bytes: usize,
    pub total_output_bytes: usize,
    pub total_bytes: usize,
    pub estimated_input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub estimated_total_tokens: usize,
    pub overall_latency: Option<LatencyStats>,
    pub tool_call_latency: Option<LatencyStats>,
    pub tools: Vec<ToolSummary>,
    pub methods: Vec<MethodSummary>,
}

// ---------------------------------------------------------------------------
// Report & session types
// ---------------------------------------------------------------------------

/// Complete benchmark report for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_secs: f64,
    pub backends: Vec<BackendSummary>,
    pub records: Vec<CallRecord>,
}

/// Lightweight session descriptor (runtime state, not the full store).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSession {
    /// Unique session identifier (UUID v4).
    pub id: String,
    /// Human-readable label supplied at session creation.
    pub name: String,
    /// The proxy backend this session is associated with.
    pub backend_name: String,
    /// Wall-clock start time.
    pub started_at: DateTime<Utc>,
    /// Whether the session is still accepting new records.
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Report comparison
// ---------------------------------------------------------------------------

/// Delta statistics for a single latency metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDelta {
    /// Absolute change in milliseconds (b − a).
    pub delta_ms: f64,
    /// Relative change as a percentage (100 * (b − a) / a), or `None` if `a == 0`.
    pub delta_pct: Option<f64>,
}

impl LatencyDelta {
    fn compute(a: f64, b: f64) -> Self {
        let delta_ms = b - a;
        let delta_pct = if a.abs() > f64::EPSILON {
            Some(100.0 * delta_ms / a)
        } else {
            None
        };
        Self { delta_ms, delta_pct }
    }
}

/// Comparison of p50/p95/p99 latency between two reports for one backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendLatencyComparison {
    pub backend_name: String,
    pub p50: LatencyDelta,
    pub p95: LatencyDelta,
    pub p99: LatencyDelta,
    pub mean: LatencyDelta,
}

/// Throughput comparison (total bytes / duration) between two reports for one backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputComparison {
    pub backend_name: String,
    /// Bytes per second in report A.
    pub bytes_per_sec_a: f64,
    /// Bytes per second in report B.
    pub bytes_per_sec_b: f64,
    /// Relative throughput change as a percentage (100 * (b − a) / a), or `None` if a == 0.
    pub delta_pct: Option<f64>,
}

/// Success-rate comparison between two reports for one backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessRateComparison {
    pub backend_name: String,
    /// Success rate percentage in report A.
    pub success_rate_a: f64,
    /// Success rate percentage in report B.
    pub success_rate_b: f64,
    /// Absolute delta in percentage points (b − a).
    pub delta_pct_points: f64,
}

/// Top-level comparison result between two [`BenchmarkReport`]s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportComparison {
    pub session_id_a: String,
    pub session_id_b: String,
    pub latency: Vec<BackendLatencyComparison>,
    pub throughput: Vec<ThroughputComparison>,
    pub success_rates: Vec<SuccessRateComparison>,
}

/// Compare two benchmark reports and compute per-backend deltas.
pub fn compare_reports(a: &BenchmarkReport, b: &BenchmarkReport) -> ReportComparison {
    // Build lookup maps keyed by backend name.
    let map_a: HashMap<&str, &BackendSummary> =
        a.backends.iter().map(|b| (b.name.as_str(), b)).collect();
    let map_b: HashMap<&str, &BackendSummary> =
        b.backends.iter().map(|b| (b.name.as_str(), b)).collect();

    // Union of all backend names across both reports.
    let mut names: Vec<&str> = map_a.keys().chain(map_b.keys()).copied().collect();
    names.sort_unstable();
    names.dedup();

    let mut latency = Vec::new();
    let mut throughput = Vec::new();
    let mut success_rates = Vec::new();

    for name in names {
        let sa = map_a.get(name);
        let sb = map_b.get(name);

        // Latency comparison (use zeroed stats when a backend is absent in one report).
        let zero_lat = LatencyStats {
            count: 0,
            min_ms: 0.0,
            max_ms: 0.0,
            mean_ms: 0.0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            std_dev_ms: 0.0,
        };
        let lat_a = sa
            .and_then(|s| s.overall_latency.as_ref())
            .unwrap_or(&zero_lat);
        let lat_b = sb
            .and_then(|s| s.overall_latency.as_ref())
            .unwrap_or(&zero_lat);

        latency.push(BackendLatencyComparison {
            backend_name: name.to_string(),
            p50: LatencyDelta::compute(lat_a.p50_ms, lat_b.p50_ms),
            p95: LatencyDelta::compute(lat_a.p95_ms, lat_b.p95_ms),
            p99: LatencyDelta::compute(lat_a.p99_ms, lat_b.p99_ms),
            mean: LatencyDelta::compute(lat_a.mean_ms, lat_b.mean_ms),
        });

        // Throughput comparison.
        let bytes_a = sa.map_or(0, |s| s.total_bytes) as f64;
        let bytes_b = sb.map_or(0, |s| s.total_bytes) as f64;
        let bps_a = if a.duration_secs > 0.0 {
            bytes_a / a.duration_secs
        } else {
            0.0
        };
        let bps_b = if b.duration_secs > 0.0 {
            bytes_b / b.duration_secs
        } else {
            0.0
        };
        let tp_delta_pct = if bps_a.abs() > f64::EPSILON {
            Some(100.0 * (bps_b - bps_a) / bps_a)
        } else {
            None
        };
        throughput.push(ThroughputComparison {
            backend_name: name.to_string(),
            bytes_per_sec_a: bps_a,
            bytes_per_sec_b: bps_b,
            delta_pct: tp_delta_pct,
        });

        // Success-rate comparison.
        let sr_a = sa.map_or(0.0, |s| s.success_rate);
        let sr_b = sb.map_or(0.0, |s| s.success_rate);
        success_rates.push(SuccessRateComparison {
            backend_name: name.to_string(),
            success_rate_a: sr_a,
            success_rate_b: sr_b,
            delta_pct_points: sr_b - sr_a,
        });
    }

    ReportComparison {
        session_id_a: a.session_id.clone(),
        session_id_b: b.session_id.clone(),
        latency,
        throughput,
        success_rates,
    }
}

// ---------------------------------------------------------------------------
// MetricsStore
// ---------------------------------------------------------------------------

/// In-memory collector for call records belonging to one benchmark session.
///
/// Once [`MAX_RECORDS`] is reached new records are silently dropped and counted
/// in `dropped` so callers can surface a warning. The store is keyed externally
/// by a session UUID and wrapped in an `Arc<Mutex<_>>` for shared access.
pub struct MetricsStore {
    pub records: Vec<CallRecord>,
    pub session_start: Instant,
    pub session_start_utc: DateTime<Utc>,
    pub dropped: usize,
}

impl MetricsStore {
    /// Create a fresh store, setting the session start time to now.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            session_start: Instant::now(),
            session_start_utc: Utc::now(),
            dropped: 0,
        }
    }

    /// Append a record, dropping it (and incrementing the counter) when the
    /// cap is reached.
    pub fn record(&mut self, rec: CallRecord) {
        if self.records.len() >= MAX_RECORDS {
            self.dropped += 1;
            return;
        }
        self.records.push(rec);
    }

    /// Build a [`CallRecord`] from timing and outcome information.
    ///
    /// This is a static helper so callers can construct records without holding
    /// a mutable borrow on the store.
    pub fn build_record(
        backend: &str,
        method: &str,
        tool_name: Option<&str>,
        resource_uri: Option<&str>,
        prompt_name: Option<&str>,
        start: Instant,
        request_bytes: usize,
        result: &Result<serde_json::Value, String>,
    ) -> CallRecord {
        let latency = start.elapsed();
        let (success, response_bytes, error_msg) = match result {
            Ok(v) => (
                true,
                serde_json::to_string(v).unwrap_or_default().len(),
                None,
            ),
            Err(e) => (false, 0, Some(e.clone())),
        };

        CallRecord {
            backend: backend.to_string(),
            method: method.to_string(),
            tool_name: tool_name.map(String::from),
            resource_uri: resource_uri.map(String::from),
            prompt_name: prompt_name.map(String::from),
            latency_us: latency.as_micros().min(u128::from(u64::MAX)) as u64,
            request_bytes,
            response_bytes,
            estimated_input_tokens: estimate_tokens(request_bytes),
            estimated_output_tokens: estimate_tokens(response_bytes),
            success,
            error_message: error_msg,
            timestamp: Utc::now(),
        }
    }

    /// Generate a full [`BenchmarkReport`] from all collected records.
    ///
    /// Can be called at any time — the session does not need to be stopped
    /// first.
    pub fn generate_report(&self, session_id: &str) -> BenchmarkReport {
        if self.dropped > 0 {
            tracing::warn!(
                session_id = session_id,
                dropped = self.dropped,
                "Benchmark records dropped due to MAX_RECORDS cap ({})",
                MAX_RECORDS
            );
        }

        let now = Utc::now();
        let duration = self.session_start.elapsed();

        // Group records by backend name.
        let mut by_backend: HashMap<String, Vec<&CallRecord>> = HashMap::new();
        for r in &self.records {
            by_backend.entry(r.backend.clone()).or_default().push(r);
        }

        let mut backends: Vec<BackendSummary> = by_backend
            .into_iter()
            .map(|(name, recs)| Self::summarize_backend(&name, &recs))
            .collect();
        // Deterministic ordering.
        backends.sort_by(|a, b| a.name.cmp(&b.name));

        BenchmarkReport {
            session_id: session_id.to_string(),
            started_at: self.session_start_utc,
            ended_at: now,
            duration_secs: duration.as_secs_f64(),
            backends,
            records: self.records.clone(),
        }
    }

    /// Build an aggregate [`BackendSummary`] from a slice of records.
    fn summarize_backend(name: &str, records: &[&CallRecord]) -> BackendSummary {
        let total_calls = records.len();
        let success_count = records.iter().filter(|r| r.success).count();
        let failure_count = total_calls - success_count;
        let success_rate = if total_calls > 0 {
            success_count as f64 / total_calls as f64 * 100.0
        } else {
            0.0
        };

        let total_input_bytes: usize = records.iter().map(|r| r.request_bytes).sum();
        let total_output_bytes: usize = records.iter().map(|r| r.response_bytes).sum();
        let est_in: usize = records.iter().map(|r| r.estimated_input_tokens).sum();
        let est_out: usize = records.iter().map(|r| r.estimated_output_tokens).sum();

        let tool_calls: Vec<_> = records
            .iter()
            .filter(|r| r.method == "tools/call")
            .collect();
        let total_tool_calls = tool_calls.len();

        let mut all_lat: Vec<u64> = records.iter().map(|r| r.latency_us).collect();
        let overall_latency = LatencyStats::from_latencies_us(&mut all_lat);

        let mut tc_lat: Vec<u64> = tool_calls.iter().map(|r| r.latency_us).collect();
        let tool_call_latency = LatencyStats::from_latencies_us(&mut tc_lat);

        // Per-tool breakdown.
        let mut tool_map: HashMap<String, Vec<&&CallRecord>> = HashMap::new();
        for r in &tool_calls {
            if let Some(ref tn) = r.tool_name {
                tool_map.entry(tn.clone()).or_default().push(r);
            }
        }
        let mut tools: Vec<ToolSummary> = tool_map
            .into_iter()
            .map(|(tn, recs)| {
                let mut lats: Vec<u64> = recs.iter().map(|r| r.latency_us).collect();
                ToolSummary {
                    name: tn,
                    call_count: recs.len(),
                    success_count: recs.iter().filter(|r| r.success).count(),
                    failure_count: recs.iter().filter(|r| !r.success).count(),
                    total_input_bytes: recs.iter().map(|r| r.request_bytes).sum(),
                    total_output_bytes: recs.iter().map(|r| r.response_bytes).sum(),
                    estimated_input_tokens: recs.iter().map(|r| r.estimated_input_tokens).sum(),
                    estimated_output_tokens: recs.iter().map(|r| r.estimated_output_tokens).sum(),
                    latency: LatencyStats::from_latencies_us(&mut lats),
                }
            })
            .collect();
        // Sort tools by call count descending.
        tools.sort_by(|a, b| b.call_count.cmp(&a.call_count));

        // Per-method breakdown.
        let mut method_map: HashMap<String, Vec<&CallRecord>> = HashMap::new();
        for r in records {
            method_map.entry(r.method.clone()).or_default().push(r);
        }
        let mut methods: Vec<MethodSummary> = method_map
            .into_iter()
            .map(|(m, recs)| {
                let mut lats: Vec<u64> = recs.iter().map(|r| r.latency_us).collect();
                MethodSummary {
                    method: m,
                    call_count: recs.len(),
                    total_input_bytes: recs.iter().map(|r| r.request_bytes).sum(),
                    total_output_bytes: recs.iter().map(|r| r.response_bytes).sum(),
                    estimated_input_tokens: recs.iter().map(|r| r.estimated_input_tokens).sum(),
                    estimated_output_tokens: recs.iter().map(|r| r.estimated_output_tokens).sum(),
                    latency: LatencyStats::from_latencies_us(&mut lats),
                }
            })
            .collect();
        // Sort methods by call count descending.
        methods.sort_by(|a, b| b.call_count.cmp(&a.call_count));

        BackendSummary {
            name: name.to_string(),
            total_calls,
            total_tool_calls,
            success_count,
            failure_count,
            success_rate,
            total_input_bytes,
            total_output_bytes,
            total_bytes: total_input_bytes + total_output_bytes,
            estimated_input_tokens: est_in,
            estimated_output_tokens: est_out,
            estimated_total_tokens: est_in + est_out,
            overall_latency,
            tool_call_latency,
            tools,
            methods,
        }
    }
}

impl Default for MetricsStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Shared store handle
// ---------------------------------------------------------------------------

/// Convenience alias for a shared, mutex-protected metrics store.
pub type SharedMetricsStore = Arc<Mutex<MetricsStore>>;

/// Create a new shared metrics store.
pub fn new_shared_store() -> SharedMetricsStore {
    Arc::new(Mutex::new(MetricsStore::new()))
}

/// Poison-tolerant lock helper — recovers the inner value even after a panic.
pub fn lock_store(store: &SharedMetricsStore) -> std::sync::MutexGuard<'_, MetricsStore> {
    store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- LatencyStats ---------------------------------------------------------

    #[test]
    fn latency_stats_empty_returns_none() {
        assert!(LatencyStats::from_latencies_us(&mut []).is_none());
    }

    #[test]
    fn latency_stats_single_value() {
        let stats = LatencyStats::from_latencies_us(&mut [5_000]).unwrap();
        assert_eq!(stats.count, 1);
        assert!((stats.min_ms - 5.0).abs() < f64::EPSILON);
        assert!((stats.max_ms - 5.0).abs() < f64::EPSILON);
        assert!((stats.mean_ms - 5.0).abs() < f64::EPSILON);
        assert!((stats.p50_ms - 5.0).abs() < f64::EPSILON);
        assert!((stats.p95_ms - 5.0).abs() < f64::EPSILON);
        assert!((stats.p99_ms - 5.0).abs() < f64::EPSILON);
        // N=1 → sample std_dev is 0
        assert!(stats.std_dev_ms.abs() < f64::EPSILON);
    }

    #[test]
    fn latency_stats_two_values() {
        let stats = LatencyStats::from_latencies_us(&mut [1_000, 3_000]).unwrap();
        assert_eq!(stats.count, 2);
        assert!((stats.min_ms - 1.0).abs() < f64::EPSILON);
        assert!((stats.max_ms - 3.0).abs() < f64::EPSILON);
        assert!((stats.mean_ms - 2.0).abs() < f64::EPSILON);
        // p50 with interpolation: rank = 0.5 * 1 = 0.5, interp(1000, 3000, 0.5) = 2000 µs = 2 ms
        assert!((stats.p50_ms - 2.0).abs() < f64::EPSILON);
        // Sample std_dev for [1000, 3000] µs:
        //   sqrt(((1000-2000)² + (3000-2000)²) / 1) = sqrt(2_000_000) ≈ 1414.21 µs = 1.414 ms
        assert!((stats.std_dev_ms - 1.414_213_562_373_095_1).abs() < 0.001);
    }

    #[test]
    fn latency_stats_ten_values() {
        let mut data: Vec<u64> = (1..=10).map(|i| i * 100).collect();
        let stats = LatencyStats::from_latencies_us(&mut data).unwrap();
        assert_eq!(stats.count, 10);
        assert!((stats.min_ms - 0.1).abs() < f64::EPSILON);
        assert!((stats.max_ms - 1.0).abs() < f64::EPSILON);
        assert!((stats.mean_ms - 0.55).abs() < 0.001);
    }

    // -- interpolated_percentile ---------------------------------------------

    #[test]
    fn percentile_exact_median() {
        // sorted: [10, 20, 30, 40, 50] — p50 rank = 2.0 → value 30
        let sorted = [10u64, 20, 30, 40, 50];
        assert!((interpolated_percentile(&sorted, 50.0) - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentile_exact_quartile() {
        let sorted = [10u64, 20, 30, 40, 50];
        // p25: rank = 0.25 * 4 = 1.0 → sorted[1] = 20
        assert!((interpolated_percentile(&sorted, 25.0) - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentile_interpolated() {
        let sorted = [10u64, 20, 30, 40, 50];
        // p10: rank = 0.10 * 4 = 0.4, interp(10, 20, 0.4) = 14
        assert!((interpolated_percentile(&sorted, 10.0) - 14.0).abs() < f64::EPSILON);
    }

    // -- MetricsStore ---------------------------------------------------------

    #[test]
    fn metrics_store_respects_cap() {
        let mut store = MetricsStore::new();
        let rec = make_call_record("backend-a", "tools/call");
        for _ in 0..MAX_RECORDS + 5 {
            store.record(rec.clone());
        }
        assert_eq!(store.records.len(), MAX_RECORDS);
        assert_eq!(store.dropped, 5);
    }

    #[test]
    fn generate_report_empty_store() {
        let store = MetricsStore::new();
        let report = store.generate_report("empty-session");
        assert_eq!(report.session_id, "empty-session");
        assert!(report.backends.is_empty());
        assert!(report.records.is_empty());
        assert!(report.duration_secs >= 0.0);
    }

    #[test]
    fn generate_report_aggregates_correctly() {
        let mut store = MetricsStore::new();
        for _ in 0..3 {
            store.record(CallRecord {
                backend: "backend-a".into(),
                method: "tools/call".into(),
                tool_name: Some("search".into()),
                resource_uri: None,
                prompt_name: None,
                latency_us: 1_000,
                request_bytes: 40,
                response_bytes: 80,
                estimated_input_tokens: estimate_tokens(40),
                estimated_output_tokens: estimate_tokens(80),
                success: true,
                error_message: None,
                timestamp: Utc::now(),
            });
        }
        // One failure
        store.record(CallRecord {
            backend: "backend-a".into(),
            method: "tools/call".into(),
            tool_name: Some("search".into()),
            resource_uri: None,
            prompt_name: None,
            latency_us: 2_000,
            request_bytes: 40,
            response_bytes: 0,
            estimated_input_tokens: estimate_tokens(40),
            estimated_output_tokens: 0,
            success: false,
            error_message: Some("timeout".into()),
            timestamp: Utc::now(),
        });

        let report = store.generate_report("test-session");
        assert_eq!(report.backends.len(), 1);
        let b = &report.backends[0];
        assert_eq!(b.name, "backend-a");
        assert_eq!(b.total_calls, 4);
        assert_eq!(b.success_count, 3);
        assert_eq!(b.failure_count, 1);
        assert!((b.success_rate - 75.0).abs() < 0.001);
        assert_eq!(b.total_tool_calls, 4);
        assert_eq!(b.tools.len(), 1);
        assert_eq!(b.tools[0].name, "search");
    }

    // -- build_record ---------------------------------------------------------

    #[test]
    fn build_record_success_path() {
        let start = Instant::now();
        let val = serde_json::json!({"result": "ok"});
        let rec = MetricsStore::build_record(
            "backend-x",
            "tools/call",
            Some("my_tool"),
            None,
            None,
            start,
            64,
            &Ok(val),
        );
        assert!(rec.success);
        assert!(rec.error_message.is_none());
        assert_eq!(rec.tool_name.as_deref(), Some("my_tool"));
        assert!(rec.response_bytes > 0);
        assert!(rec.latency_us < 1_000_000);
    }

    #[test]
    fn build_record_error_path() {
        let start = Instant::now();
        let rec = MetricsStore::build_record(
            "backend-x",
            "tools/call",
            Some("my_tool"),
            None,
            None,
            start,
            64,
            &Err("connection refused".into()),
        );
        assert!(!rec.success);
        assert_eq!(rec.error_message.as_deref(), Some("connection refused"));
        assert_eq!(rec.response_bytes, 0);
    }

    // -- estimate_tokens ------------------------------------------------------

    #[test]
    fn estimate_tokens_rounding() {
        assert_eq!(estimate_tokens(0), 0);
        assert_eq!(estimate_tokens(1), 1);
        assert_eq!(estimate_tokens(4), 1);
        assert_eq!(estimate_tokens(5), 2);
        assert_eq!(estimate_tokens(400), 100);
    }

    // -- compare_reports ------------------------------------------------------

    #[test]
    fn compare_reports_basic_deltas() {
        let report_a = build_report("session-a", "backend-x", 3, 1_000, 100.0);
        let report_b = build_report("session-b", "backend-x", 3, 2_000, 100.0);
        let cmp = compare_reports(&report_a, &report_b);

        assert_eq!(cmp.session_id_a, "session-a");
        assert_eq!(cmp.session_id_b, "session-b");
        assert_eq!(cmp.latency.len(), 1);
        // Mean in A ≈ 1 ms, mean in B ≈ 2 ms → positive delta
        assert!(cmp.latency[0].mean.delta_ms > 0.0);
        // delta_pct should be ~+100%
        let pct = cmp.latency[0].mean.delta_pct.unwrap();
        assert!((pct - 100.0).abs() < 1.0, "expected ~100% delta, got {}", pct);
    }

    #[test]
    fn compare_reports_missing_backend_in_b() {
        let report_a = build_report("session-a", "backend-a", 2, 500, 100.0);
        let mut report_b = build_report("session-b", "backend-a", 2, 500, 100.0);
        // Remove the backend from report_b to simulate it being absent.
        report_b.backends.clear();

        let cmp = compare_reports(&report_a, &report_b);
        assert_eq!(cmp.latency.len(), 1);
        // Absent backend in B → p50 in B is 0 → negative delta
        assert!(cmp.latency[0].p50.delta_ms <= 0.0);
    }

    // -- helpers --------------------------------------------------------------

    fn make_call_record(backend: &str, method: &str) -> CallRecord {
        CallRecord {
            backend: backend.to_string(),
            method: method.to_string(),
            tool_name: None,
            resource_uri: None,
            prompt_name: None,
            latency_us: 500,
            request_bytes: 20,
            response_bytes: 40,
            estimated_input_tokens: estimate_tokens(20),
            estimated_output_tokens: estimate_tokens(40),
            success: true,
            error_message: None,
            timestamp: Utc::now(),
        }
    }

    /// Build a minimal [`BenchmarkReport`] containing `count` identical records
    /// for `backend_name`, each with the given `latency_us`.
    fn build_report(
        session_id: &str,
        backend_name: &str,
        count: usize,
        latency_us: u64,
        success_rate: f64,
    ) -> BenchmarkReport {
        let mut store = MetricsStore::new();
        let failures = count - (count as f64 * success_rate / 100.0).round() as usize;
        for i in 0..count {
            store.record(CallRecord {
                backend: backend_name.to_string(),
                method: "tools/call".to_string(),
                tool_name: Some("test_tool".to_string()),
                resource_uri: None,
                prompt_name: None,
                latency_us,
                request_bytes: 40,
                response_bytes: 80,
                estimated_input_tokens: estimate_tokens(40),
                estimated_output_tokens: estimate_tokens(80),
                success: i >= failures,
                error_message: if i < failures {
                    Some("err".into())
                } else {
                    None
                },
                timestamp: Utc::now(),
            });
        }
        store.generate_report(session_id)
    }
}
