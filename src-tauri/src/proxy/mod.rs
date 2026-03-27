//! Proxy module - Universal MCP server proxying and adaptation

pub mod benchmark;
pub mod manager;
pub mod types;

pub use benchmark::{
    compare_reports, estimate_tokens, lock_store, new_shared_store, BackendLatencyComparison,
    BackendSummary, BenchmarkReport, BenchmarkSession, CallRecord, LatencyDelta, LatencyStats,
    MethodSummary, MetricsStore, ReportComparison, SharedMetricsStore, SuccessRateComparison,
    ThroughputComparison, ToolSummary,
};
pub use manager::ProxyManager;
pub use types::*;

#[cfg(test)]
mod tests {
    #[test]
    fn test_proxy_module_loads() {
        // Ensure module compiles
    }
}
