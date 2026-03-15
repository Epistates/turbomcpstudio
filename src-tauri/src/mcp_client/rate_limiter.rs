//! Rate Limiter Module
//!
//! Provides sliding-window rate limiting for MCP operations per server.
//! Prevents overwhelming MCP servers or LLM providers with excessive requests.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Sliding window rate limiter for MCP operations
///
/// Tracks request timestamps per server and rejects requests that
/// exceed the configured threshold within the time window.
pub struct RateLimiter {
    /// Max requests per window (0 = disabled)
    max_requests: u32,
    /// Window duration
    window: Duration,
    /// Request timestamps per server
    windows: Mutex<HashMap<Uuid, Vec<Instant>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `max_requests` - Maximum requests allowed per window (0 to disable)
    /// * `window_seconds` - Window duration in seconds
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_seconds),
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Create a disabled rate limiter (passes all requests)
    pub fn disabled() -> Self {
        Self::new(0, 60)
    }

    /// Check if a request is allowed for the given server
    ///
    /// Returns `Ok(())` if allowed, `Err(message)` if rate limit exceeded.
    /// Also records the request timestamp on success.
    pub fn check_rate_limit(&self, server_id: Uuid) -> Result<(), String> {
        if self.max_requests == 0 {
            return Ok(()); // Rate limiting disabled
        }

        let mut windows = self.windows.lock();
        let now = Instant::now();

        let timestamps = windows.entry(server_id).or_default();

        // Remove expired entries outside the window
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        if timestamps.len() >= self.max_requests as usize {
            let oldest = timestamps.first().map(|t| {
                let remaining = self.window.saturating_sub(now.duration_since(*t));
                remaining.as_secs()
            });

            Err(format!(
                "Rate limit exceeded: {}/{} requests in {}s window. Retry in ~{}s.",
                timestamps.len(),
                self.max_requests,
                self.window.as_secs(),
                oldest.unwrap_or(0)
            ))
        } else {
            timestamps.push(now);
            Ok(())
        }
    }

    /// Get current request count for a server within the window
    #[allow(dead_code)]
    pub fn current_count(&self, server_id: Uuid) -> usize {
        let mut windows = self.windows.lock();
        let now = Instant::now();

        if let Some(timestamps) = windows.get_mut(&server_id) {
            timestamps.retain(|t| now.duration_since(*t) < self.window);
            timestamps.len()
        } else {
            0
        }
    }

    /// Update rate limiter configuration
    #[allow(dead_code)]
    pub fn update_config(&mut self, max_requests: u32, window_seconds: u64) {
        self.max_requests = max_requests;
        self.window = Duration::from_secs(window_seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(5, 60);
        let server_id = Uuid::new_v4();

        for _ in 0..5 {
            assert!(limiter.check_rate_limit(server_id).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_rejects_over_limit() {
        let limiter = RateLimiter::new(3, 60);
        let server_id = Uuid::new_v4();

        for _ in 0..3 {
            assert!(limiter.check_rate_limit(server_id).is_ok());
        }

        assert!(limiter.check_rate_limit(server_id).is_err());
    }

    #[test]
    fn test_rate_limiter_disabled() {
        let limiter = RateLimiter::disabled();
        let server_id = Uuid::new_v4();

        // Should always pass
        for _ in 0..1000 {
            assert!(limiter.check_rate_limit(server_id).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_per_server_isolation() {
        let limiter = RateLimiter::new(2, 60);
        let server_a = Uuid::new_v4();
        let server_b = Uuid::new_v4();

        assert!(limiter.check_rate_limit(server_a).is_ok());
        assert!(limiter.check_rate_limit(server_a).is_ok());
        assert!(limiter.check_rate_limit(server_a).is_err()); // Server A at limit

        // Server B should still work
        assert!(limiter.check_rate_limit(server_b).is_ok());
        assert!(limiter.check_rate_limit(server_b).is_ok());
    }
}
