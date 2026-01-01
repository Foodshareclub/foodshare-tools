//! Telemetry, metrics, and observability for Foodshare tools
//!
//! This crate provides enterprise-grade observability:
//! - Structured logging with tracing
//! - Metrics collection and export
//! - Performance tracking
//! - Error reporting

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::Uuid;

/// Global metrics registry
static METRICS: Lazy<MetricsRegistry> = Lazy::new(MetricsRegistry::new);

/// Global session ID for correlating logs
static SESSION_ID: Lazy<String> = Lazy::new(|| Uuid::new_v4().to_string());

/// Initialize the telemetry system
pub fn init() -> anyhow::Result<()> {
    init_with_config(TelemetryConfig::default())
}

/// Initialize with custom configuration
pub fn init_with_config(config: TelemetryConfig) -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer()
            .with_target(config.show_target)
            .with_thread_ids(config.show_thread_ids)
            .with_file(config.show_file)
            .with_line_number(config.show_line_number)
            .compact());

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow::anyhow!("Failed to set tracing subscriber: {}", e))?;

    tracing::info!(
        session_id = %session_id(),
        version = env!("CARGO_PKG_VERSION"),
        "Telemetry initialized"
    );

    Ok(())
}

/// Get the current session ID
pub fn session_id() -> &'static str {
    &SESSION_ID
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub log_level: String,
    pub show_target: bool,
    pub show_thread_ids: bool,
    pub show_file: bool,
    pub show_line_number: bool,
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

/// Metrics registry for collecting and exporting metrics
pub struct MetricsRegistry {
    counters: RwLock<HashMap<String, AtomicU64>>,
    gauges: RwLock<HashMap<String, AtomicU64>>,
    histograms: RwLock<HashMap<String, Vec<f64>>>,
    start_time: Instant,
}

impl MetricsRegistry {
    fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            start_time: Instant::now(),
        }
    }

    /// Increment a counter
    pub fn increment(&self, name: &str) {
        self.increment_by(name, 1);
    }

    /// Increment a counter by a specific amount
    pub fn increment_by(&self, name: &str, value: u64) {
        let counters = self.counters.read().unwrap();
        if let Some(counter) = counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        } else {
            drop(counters);
            let mut counters = self.counters.write().unwrap();
            counters
                .entry(name.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Set a gauge value
    pub fn gauge(&self, name: &str, value: u64) {
        let mut gauges = self.gauges.write().unwrap();
        gauges
            .entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .store(value, Ordering::Relaxed);
    }

    /// Record a histogram value
    pub fn histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().unwrap();
        histograms
            .entry(name.to_string())
            .or_default()
            .push(value);
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Export metrics as JSON
    pub fn export_json(&self) -> serde_json::Value {
        let counters = self.counters.read().unwrap();
        let gauges = self.gauges.read().unwrap();
        let histograms = self.histograms.read().unwrap();

        let counter_values: HashMap<String, u64> = counters
            .iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect();

        let gauge_values: HashMap<String, u64> = gauges
            .iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect();

        let histogram_stats: HashMap<String, HistogramStats> = histograms
            .iter()
            .map(|(k, v)| (k.clone(), HistogramStats::from_values(v)))
            .collect();

        serde_json::json!({
            "session_id": session_id(),
            "uptime_secs": self.uptime_secs(),
            "counters": counter_values,
            "gauges": gauge_values,
            "histograms": histogram_stats,
        })
    }
}

/// Histogram statistics
#[derive(Debug, Serialize)]
pub struct HistogramStats {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

impl HistogramStats {
    fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self {
                count: 0,
                min: 0.0,
                max: 0.0,
                mean: 0.0,
                p50: 0.0,
                p95: 0.0,
                p99: 0.0,
            };
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let count = sorted.len();
        let sum: f64 = sorted.iter().sum();

        Self {
            count,
            min: sorted[0],
            max: sorted[count - 1],
            mean: sum / count as f64,
            p50: percentile(&sorted, 50.0),
            p95: percentile(&sorted, 95.0),
            p99: percentile(&sorted, 99.0),
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Get the global metrics registry
pub fn metrics() -> &'static MetricsRegistry {
    &METRICS
}

/// Timer for measuring operation duration
pub struct Timer {
    name: String,
    start: Instant,
}

impl Timer {
    /// Start a new timer
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
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub event_type: String,
    pub data: serde_json::Value,
}

impl Event {
    pub fn new(event_type: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            timestamp: Utc::now(),
            session_id: session_id().to_string(),
            event_type: event_type.into(),
            data,
        }
    }

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
        let registry = MetricsRegistry::new();
        registry.increment("test_counter");
        registry.increment("test_counter");
        registry.increment_by("test_counter", 3);

        let counters = registry.counters.read().unwrap();
        assert_eq!(counters.get("test_counter").unwrap().load(Ordering::Relaxed), 5);
    }

    #[test]
    fn test_metrics_gauge() {
        let registry = MetricsRegistry::new();
        registry.gauge("test_gauge", 42);
        registry.gauge("test_gauge", 100);

        let gauges = registry.gauges.read().unwrap();
        assert_eq!(gauges.get("test_gauge").unwrap().load(Ordering::Relaxed), 100);
    }

    #[test]
    fn test_histogram_stats() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let stats = HistogramStats::from_values(&values);

        assert_eq!(stats.count, 10);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 10.0);
        assert_eq!(stats.mean, 5.5);
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
}
