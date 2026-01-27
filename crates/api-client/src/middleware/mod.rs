//! Middleware components for request/response processing
//!
//! This module re-exports the resilience components from `foodshare-core`.

// Re-export from foodshare-core for convenience
pub use foodshare_core::rate_limit::{RateLimitConfig, RateLimitStatus, RateLimiter};
pub use foodshare_core::retry::{
    retry, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState, RetryConfig,
    RetryResult,
};
