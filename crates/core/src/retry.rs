//! Retry logic with exponential backoff
//!
//! Provides robust retry mechanisms for flaky operations:
//! - Exponential backoff with jitter
//! - Configurable retry policies
//! - Circuit breaker pattern
//!
//! # Example
//!
//! ```rust,no_run
//! use foodshare_core::retry::{retry, RetryConfig};
//!
//! let result = retry(RetryConfig::default(), || {
//!     // Potentially flaky operation
//!     Ok::<_, std::io::Error>("success")
//! });
//! ```

use crate::error::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Add random jitter to delays
    pub jitter: bool,
    /// Timeout for each attempt
    pub attempt_timeout: Option<Duration>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: true,
            attempt_timeout: None,
        }
    }
}

impl RetryConfig {
    /// Create a config for quick retries
    pub fn quick() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_millis(500),
            backoff_multiplier: 2.0,
            jitter: true,
            attempt_timeout: Some(Duration::from_secs(5)),
        }
    }

    /// Create a config for patient retries
    pub fn patient() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
            attempt_timeout: Some(Duration::from_secs(60)),
        }
    }

    /// Create a config with no retries
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            backoff_multiplier: 1.0,
            jitter: false,
            attempt_timeout: None,
        }
    }

    /// Calculate delay for a given attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let base_delay = self.initial_delay.as_secs_f64()
            * self.backoff_multiplier.powi(attempt as i32 - 1);

        let delay_secs = base_delay.min(self.max_delay.as_secs_f64());

        let final_delay = if self.jitter {
            // Add up to 25% jitter
            let jitter_factor = 1.0 + (rand_simple() * 0.25);
            delay_secs * jitter_factor
        } else {
            delay_secs
        };

        Duration::from_secs_f64(final_delay)
    }
}

/// Simple pseudo-random number generator (0.0 to 1.0)
fn rand_simple() -> f64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    );
    (hasher.finish() % 1000) as f64 / 1000.0
}

/// Retry result with attempt information
#[derive(Debug)]
pub struct RetryResult<T> {
    /// The successful result
    pub value: T,
    /// Number of attempts made
    pub attempts: u32,
    /// Total time spent retrying
    pub total_duration: Duration,
}

/// Execute a function with retry logic
pub fn retry<F, T, E>(config: RetryConfig, mut f: F) -> std::result::Result<RetryResult<T>, E>
where
    F: FnMut() -> std::result::Result<T, E>,
    E: std::fmt::Display,
{
    let start = Instant::now();
    let mut last_error: Option<E> = None;

    for attempt in 0..config.max_attempts {
        // Wait before retry (except first attempt)
        if attempt > 0 {
            let delay = config.delay_for_attempt(attempt);
            thread::sleep(delay);
        }

        match f() {
            Ok(value) => {
                return Ok(RetryResult {
                    value,
                    attempts: attempt + 1,
                    total_duration: start.elapsed(),
                });
            }
            Err(e) => {
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap())
}

/// Execute with retry, returning our Result type
pub fn retry_operation<F, T>(config: RetryConfig, operation_name: &str, f: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    match retry(config.clone(), f) {
        Ok(result) => Ok(result.value),
        Err(e) => Err(Error::new(
            ErrorCode::ProcessError,
            format!(
                "{} failed after {} attempts: {}",
                operation_name, config.max_attempts, e
            ),
        )),
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: std::sync::RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Number of successes in half-open to close circuit
    pub success_threshold: u32,
    /// Time to wait before trying half-open
    pub reset_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            reset_timeout: Duration::from_secs(30),
        }
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: std::sync::RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitState {
        *self.state.read().unwrap()
    }

    /// Check if circuit allows execution
    pub fn can_execute(&self) -> bool {
        let state = self.state();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should try half-open
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if now - last_failure >= self.config.reset_timeout.as_secs() {
                    // Transition to half-open
                    if let Ok(mut guard) = self.state.write() {
                        *guard = CircuitState::HalfOpen;
                        self.success_count.store(0, Ordering::Relaxed);
                    }
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful execution
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);

        let state = self.state();
        if state == CircuitState::HalfOpen {
            let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
            if successes >= self.config.success_threshold {
                if let Ok(mut guard) = self.state.write() {
                    *guard = CircuitState::Closed;
                }
            }
        }
    }

    /// Record a failed execution
    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        self.last_failure_time.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::Relaxed,
        );

        let state = self.state();
        match state {
            CircuitState::Closed => {
                if failures >= self.config.failure_threshold {
                    if let Ok(mut guard) = self.state.write() {
                        *guard = CircuitState::Open;
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open goes back to open
                if let Ok(mut guard) = self.state.write() {
                    *guard = CircuitState::Open;
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Execute with circuit breaker protection
    pub fn execute<F, T, E>(&self, f: F) -> std::result::Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> std::result::Result<T, E>,
    {
        if !self.can_execute() {
            return Err(CircuitBreakerError::CircuitOpen);
        }

        match f() {
            Ok(value) => {
                self.record_success();
                Ok(value)
            }
            Err(e) => {
                self.record_failure();
                Err(CircuitBreakerError::ExecutionFailed(e))
            }
        }
    }

    /// Reset the circuit breaker
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        if let Ok(mut guard) = self.state.write() {
            *guard = CircuitState::Closed;
        }
    }
}

/// Circuit breaker error
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    CircuitOpen,
    ExecutionFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::ExecutionFailed(e) => write!(f, "Execution failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CircuitBreakerError::CircuitOpen => None,
            CircuitBreakerError::ExecutionFailed(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_success_first_attempt() {
        let config = RetryConfig::default();
        let result = retry(config, || Ok::<_, &str>("success")).unwrap();

        assert_eq!(result.value, "success");
        assert_eq!(result.attempts, 1);
    }

    #[test]
    fn test_retry_success_after_failures() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            ..Default::default()
        };

        let mut attempt = 0;
        let result = retry(config, || {
            attempt += 1;
            if attempt < 3 {
                Err("not yet")
            } else {
                Ok("success")
            }
        })
        .unwrap();

        assert_eq!(result.value, "success");
        assert_eq!(result.attempts, 3);
    }

    #[test]
    fn test_retry_all_failures() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            ..Default::default()
        };

        let result = retry(config, || Err::<(), _>("always fails"));
        assert!(result.is_err());
    }

    #[test]
    fn test_delay_calculation() {
        let config = RetryConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: false,
            ..Default::default()
        };

        assert_eq!(config.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(400));
    }

    #[test]
    fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}
