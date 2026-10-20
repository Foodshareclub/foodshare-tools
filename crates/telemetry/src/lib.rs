//! Telemetry, metrics, and observability for Foodshare tools
//!
//! This crate provides enterprise-grade observability:
//! - Structured logging with tracing
//! - Metrics collection, backed by a Prometheus registry
//! - Performance tracking
//! - Error reporting
//!
//! The metrics registry is the single source of truth. It renders to the
//! Prometheus text exposition format for scraping, and to JSON for the
//! status endpoint.
//!
//! # Example
//!
//! ```rust
//! use foodshare_telemetry::metrics;
//!
//! metrics().increment("files_processed");
//! metrics().gauge("active_jobs", 2);
//! metrics().histogram("processing_ms", 12.5);
//!
//! let prometheus = foodshare_telemetry::prometheus::render();
//! assert!(prometheus.contains("files_processed"));
//! ```

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use uuid::Uuid;

pub mod metrics;
pub mod prometheus;

pub use metrics::{HistogramStats, Telemetry};

/// Global metrics registry
static METRICS: Lazy<Telemetry> = Lazy::new(Telemetry::new);

/// Global session ID for correlating logs
static SESSION_ID: Lazy<String> = Lazy::new(|| Uuid::new_v4().to_string());

/// Initialize the telemetry system
pub fn init() -> anyhow::Result<()> {
    init_with_config(TelemetryConfig::default())
}

/// Initialize with custom configuration
pub fn init_with_config(config: TelemetryConfig) -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let subscriber = tracing_subscriber::registry().with(filter).with(
        fmt::layer()
            .with_target(config.show_target)
            .with_thread_ids(config.show_thread_ids)
            .with_file(config.show_file)
            .with_line_number(config.show_line_number)
            .compact(),
    );

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow::anyhow!("Failed to set tracing subscriber: {}", e))?;

    tracing::info!(
        session_id = %session_id(),
        version = env!("CARGO_PKG_VERSION"),
        "Telemetry initialized"
    );

    // The registry is created on first use, and its uptime is measured
    // from that moment. Creating it here makes the uptime the process's
    // uptime rather than the time since the first scrape.
    let _ = metrics();

    Ok(())
}

/// Get the current session ID
pub fn session_id() -> &'static str {
    &SESSION_ID
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Minimum log level to emit.
    pub log_level: String,
    /// Whether the log target is shown.
    pub show_target: bool,
    /// Whether thread ids are shown.
    pub show_thread_ids: bool,
    /// Whether the source file is shown.
    pub show_file: bool,
    /// Whether the source line is shown.
    pub show_line_number: bool,
    /// Whether metrics collection is enabled.
    pub metrics_enabled: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            show_target: false,
            show_thread_ids: false,
            show_file: false,
            show_line_number: false,
            metrics_enabled: true,
        }
    }
}

/// Get the global metrics registry
pub fn metrics() -> &'static Telemetry {
    &METRICS
}

/// Timer for measuring operation duration
pub struct Timer {
    name: String,
    start: Instant,
}

impl Timer {
    /// Start a new timer
    #[must_use]
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
        }
    }

    /// Stop the timer and record the duration
    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();
        metrics().histogram(&self.name, duration.as_secs_f64() * 1000.0);
        tracing::debug!(
            metric = %self.name,
            duration_ms = duration.as_millis(),
            "Timer completed"
        );
        duration
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        // Record duration if not explicitly stopped
        let duration = self.start.elapsed();
        metrics().histogram(&self.name, duration.as_secs_f64() * 1000.0);
    }
}

/// Span for tracing operations
#[macro_export]
macro_rules! timed_span {
    ($name:expr) => {
        let _timer = $crate::Timer::start($name);
        let _span = tracing::info_span!($name).entered();
    };
    ($name:expr, $($field:tt)*) => {
        let _timer = $crate::Timer::start($name);
        let _span = tracing::info_span!($name, $($field)*).entered();
    };
}

/// Event for structured logging
#[derive(Debug, Serialize)]
pub struct Event {
    /// When the event happened.
    pub timestamp: DateTime<Utc>,
    /// The session the event belongs to.
    pub session_id: String,
    /// The event type.
    pub event_type: String,
    /// The event payload.
    pub data: serde_json::Value,
}

impl Event {
    /// Create a new event.
    pub fn new(event_type: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            timestamp: Utc::now(),
            session_id: session_id().to_string(),
            event_type: event_type.into(),
            data,
        }
    }

    /// Emit the event to the log.
    pub fn log(&self) {
        tracing::info!(
            event_type = %self.event_type,
            data = %self.data,
            "Event recorded"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counter() {
        let registry = Telemetry::new();
        registry.increment("test_counter");
        registry.increment("test_counter");
        registry.increment_by("test_counter", 3);

        assert_eq!(registry.counters().get("test_counter").copied(), Some(5));
    }

    #[test]
    fn test_metrics_gauge() {
        let registry = Telemetry::new();
        registry.gauge("test_gauge", 42);
        registry.gauge("test_gauge", 100);
        assert_eq!(registry.gauges().get("test_gauge").copied(), Some(100));
    }

    #[test]
    fn test_histogram_stats() {
        let registry = Telemetry::new();
        for value in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0] {
            registry.histogram("test_histogram", value);
        }
        let stats = registry.histograms();
        let stats = stats.get("test_histogram").expect("histogram registered");
        assert_eq!(stats.count, 10);
        assert!((stats.mean - 5.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = timer.stop();
        assert!(duration.as_millis() >= 10);
    }

    #[test]
    fn test_session_id() {
        let id = session_id();
        assert!(!id.is_empty());
        // Should be a valid UUID
        assert!(Uuid::parse_str(id).is_ok());
    }

    #[test]
    fn test_event_log() {
        let event = Event::new("test_event", serde_json::json!({"key": "value"}));
        assert_eq!(event.event_type, "test_event");
        assert!(!event.session_id.is_empty());
    }
}
