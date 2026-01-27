//! Rate limiting for API calls and resource access
//!
//! Provides token bucket and sliding window rate limiters for:
//! - API call throttling
//! - Resource access control
//! - Burst handling
//!
//! # Example
//!
//! ```rust,ignore
//! use foodshare_core::rate_limit::{RateLimiter, RateLimitConfig};
//!
//! let limiter = RateLimiter::new(RateLimitConfig::default());
//!
//! if limiter.try_acquire("api_call") {
//!     // Proceed with API call
//! } else {
//!     // Rate limited, wait or reject
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Rate limiter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window duration
    pub window: Duration,
    /// Burst allowance (extra requests allowed in short bursts)
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            burst: 10,
        }
    }
}

impl RateLimitConfig {
    /// Create a strict rate limit (no burst)
    #[must_use] pub fn strict(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            burst: 0,
        }
    }

    /// Create a lenient rate limit with burst
    #[must_use] pub fn lenient(max_requests: u32, window: Duration, burst: u32) -> Self {
        Self {
            max_requests,
            window,
            burst,
        }
    }

    /// Per-second rate limit
    #[must_use] pub fn per_second(max: u32) -> Self {
        Self {
            max_requests: max,
            window: Duration::from_secs(1),
            burst: max / 2,
        }
    }

    /// Per-minute rate limit
    #[must_use] pub fn per_minute(max: u32) -> Self {
        Self {
            max_requests: max,
            window: Duration::from_secs(60),
            burst: max / 4,
        }
    }
}

/// Token bucket state
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_update: Instant,
    config: RateLimitConfig,
}

impl TokenBucket {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            tokens: f64::from(config.max_requests + config.burst),
            last_update: Instant::now(),
            config,
        }
    }

    fn try_acquire(&mut self, tokens: u32) -> bool {
        self.refill();

        if self.tokens >= f64::from(tokens) {
            self.tokens -= f64::from(tokens);
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        let refill_rate = f64::from(self.config.max_requests) / self.config.window.as_secs_f64();
        let new_tokens = elapsed.as_secs_f64() * refill_rate;

        self.tokens = (self.tokens + new_tokens)
            .min(f64::from(self.config.max_requests + self.config.burst));
        self.last_update = now;
    }

    fn available(&mut self) -> u32 {
        self.refill();
        self.tokens as u32
    }

    fn time_until_available(&mut self, tokens: u32) -> Duration {
        self.refill();

        if self.tokens >= f64::from(tokens) {
            return Duration::ZERO;
        }

        let needed = f64::from(tokens) - self.tokens;
        let refill_rate = f64::from(self.config.max_requests) / self.config.window.as_secs_f64();
        Duration::from_secs_f64(needed / refill_rate)
    }
}

/// Rate limiter with multiple buckets
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    default_config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter
    #[must_use] pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            default_config: config,
        }
    }

    /// Try to acquire a token for the given key
    #[must_use] pub fn try_acquire(&self, key: &str) -> bool {
        self.try_acquire_n(key, 1)
    }

    /// Try to acquire multiple tokens
    #[must_use] pub fn try_acquire_n(&self, key: &str, tokens: u32) -> bool {
        // Handle poisoned lock by recovering the data (still valid even after panic)
        let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_config.clone()));
        bucket.try_acquire(tokens)
    }

    /// Get available tokens for a key
    #[must_use] pub fn available(&self, key: &str) -> u32 {
        let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_config.clone()));
        bucket.available()
    }

    /// Get time until tokens are available
    #[must_use] pub fn time_until_available(&self, key: &str, tokens: u32) -> Duration {
        let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_config.clone()));
        bucket.time_until_available(tokens)
    }

    /// Reset rate limit for a key
    pub fn reset(&self, key: &str) {
        let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        buckets.remove(key);
    }

    /// Reset all rate limits
    pub fn reset_all(&self) {
        let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        buckets.clear();
    }

    /// Get rate limit status
    #[must_use] pub fn status(&self, key: &str) -> RateLimitStatus {
        let mut buckets = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_config.clone()));

        RateLimitStatus {
            available: bucket.available(),
            max: self.default_config.max_requests + self.default_config.burst,
            reset_in: bucket.time_until_available(self.default_config.max_requests),
        }
    }
}

/// Rate limit status
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStatus {
    /// Available tokens
    pub available: u32,
    /// Maximum tokens
    pub max: u32,
    /// Time until full reset
    pub reset_in: Duration,
}

/// Sliding window rate limiter (more accurate but uses more memory)
pub struct SlidingWindowLimiter {
    windows: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    config: RateLimitConfig,
}

impl SlidingWindowLimiter {
    /// Create a new sliding window limiter
    #[must_use] pub fn new(config: RateLimitConfig) -> Self {
        Self {
            windows: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Try to acquire permission
    #[must_use] pub fn try_acquire(&self, key: &str) -> bool {
        let mut windows = self.windows.write().unwrap_or_else(|e| e.into_inner());
        let window = windows.entry(key.to_string()).or_default();

        let now = Instant::now();
        // Use saturating subtraction to avoid panic on underflow
        let cutoff = now.checked_sub(self.config.window).unwrap_or(Instant::now());

        // Remove old entries
        window.retain(|&t| t > cutoff);

        if window.len() < self.config.max_requests as usize {
            window.push(now);
            true
        } else {
            false
        }
    }

    /// Get current request count in window
    #[must_use] pub fn current_count(&self, key: &str) -> usize {
        let mut windows = self.windows.write().unwrap_or_else(|e| e.into_inner());
        let window = windows.entry(key.to_string()).or_default();

        let cutoff = Instant::now().checked_sub(self.config.window).unwrap_or(Instant::now());
        window.retain(|&t| t > cutoff);
        window.len()
    }

    /// Reset for a key
    pub fn reset(&self, key: &str) {
        let mut windows = self.windows.write().unwrap_or_else(|e| e.into_inner());
        windows.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_basic() {
        let config = RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(1),
            burst: 0,
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.try_acquire("test"));
        assert!(limiter.try_acquire("test"));
        assert!(limiter.try_acquire("test"));
        assert!(!limiter.try_acquire("test")); // Should be rate limited
    }

    #[test]
    fn test_rate_limiter_with_burst() {
        let config = RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(1),
            burst: 2,
        };
        let limiter = RateLimiter::new(config);

        // Should allow max + burst = 4 requests
        assert!(limiter.try_acquire("test"));
        assert!(limiter.try_acquire("test"));
        assert!(limiter.try_acquire("test"));
        assert!(limiter.try_acquire("test"));
        assert!(!limiter.try_acquire("test"));
    }

    #[test]
    fn test_rate_limiter_different_keys() {
        let config = RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(1),
            burst: 0,
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.try_acquire("key1"));
        assert!(!limiter.try_acquire("key1"));
        assert!(limiter.try_acquire("key2")); // Different key should work
    }

    #[test]
    fn test_rate_limiter_reset() {
        let config = RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(1),
            burst: 0,
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.try_acquire("test"));
        assert!(!limiter.try_acquire("test"));

        limiter.reset("test");
        assert!(limiter.try_acquire("test"));
    }

    #[test]
    fn test_sliding_window_basic() {
        let config = RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(1),
            burst: 0,
        };
        let limiter = SlidingWindowLimiter::new(config);

        assert!(limiter.try_acquire("test"));
        assert!(limiter.try_acquire("test"));
        assert!(!limiter.try_acquire("test"));
    }

    #[test]
    fn test_status() {
        let config = RateLimitConfig {
            max_requests: 10,
            window: Duration::from_secs(60),
            burst: 5,
        };
        let limiter = RateLimiter::new(config);

        let status = limiter.status("test");
        assert_eq!(status.max, 15); // max + burst
        assert_eq!(status.available, 15);
    }
}
