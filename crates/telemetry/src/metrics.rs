//! Metric collection, backed by a Prometheus registry.
//!
//! The registry is the single source of truth for every metric the tools
//! record. Values are exposed two ways:
//!
//! - [`Telemetry::render_prometheus`] — the Prometheus text exposition
//!   format, for scraping.
//! - [`Telemetry::export_json`] — a JSON snapshot, for the status endpoint
//!   and for tests.
//!
//! Metric names are chosen at the call site, so they are sanitised to the
//! character set Prometheus accepts before they reach the registry.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

use prometheus::core::Collector;
use prometheus::{
    Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};
use serde::Serialize;

/// Bucket boundaries, in milliseconds, for duration histograms.
///
/// The Prometheus defaults assume seconds; the tools measure in
/// milliseconds, so the buckets are chosen for that unit.
const DURATION_BUCKETS_MS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1_000.0, 2_500.0, 5_000.0, 10_000.0,
];

/// A metric registered in the registry.
enum Entry {
    /// A monotonically increasing counter.
    Counter(IntCounter),
    /// A value that can move in either direction.
    Gauge(IntGauge),
    /// A distribution of observations.
    Histogram(Histogram),
}

/// Summary statistics for a histogram.
#[derive(Debug, Clone, Serialize)]
pub struct HistogramStats {
    /// Number of observations recorded.
    pub count: u64,
    /// Lower bound of the lowest populated bucket.
    pub min: f64,
    /// Upper bound of the highest populated bucket.
    pub max: f64,
    /// Arithmetic mean of the observations.
    pub mean: f64,
    /// 50th percentile, interpolated within its bucket.
    pub p50: f64,
    /// 95th percentile, interpolated within its bucket.
    pub p95: f64,
    /// 99th percentile, interpolated within its bucket.
    pub p99: f64,
}

/// The name of the gauge that always reports the process uptime.
pub const UPTIME_METRIC: &str = "foodshare_service_uptime_seconds";

/// The name of the counter that counts HTTP requests.
pub const HTTP_REQUESTS_METRIC: &str = "foodshare_http_requests_total";

/// The name of the histogram that measures HTTP request duration.
pub const HTTP_DURATION_METRIC: &str = "foodshare_http_request_duration_seconds";

/// The buckets for HTTP request duration, in seconds.
pub const HTTP_DURATION_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0,
];

/// The metric registry.
///
/// Cheap to share: the type is `Sync`, and the handles are cloned out of
/// it, so a caller can hold one for the life of the process.
pub struct Telemetry {
    registry: Registry,
    metrics: RwLock<HashMap<String, Entry>>,
    label_counters: RwLock<HashMap<String, IntCounterVec>>,
    label_histograms: RwLock<HashMap<String, HistogramVec>>,
    start_time: Instant,
}

impl Telemetry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        let telemetry = Self {
            registry: Registry::new(),
            metrics: RwLock::new(HashMap::new()),
            label_counters: RwLock::new(HashMap::new()),
            label_histograms: RwLock::new(HashMap::new()),
            start_time: Instant::now(),
        };
        // A scrape target with nothing in it is hard to tell apart from a
        // broken one, so the uptime gauge is always present.
        telemetry.gauge(UPTIME_METRIC, 0);
        telemetry
    }

    /// The underlying Prometheus registry.
    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Seconds since the registry was created.
    #[must_use]
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Increment a counter by one.
    ///
    /// The counter is created on first use. A name already in use by a
    /// different metric type is left alone.
    pub fn increment(&self, name: &str) {
        self.increment_by(name, 1);
    }

    /// Increment a counter by a specific amount.
    pub fn increment_by(&self, name: &str, value: u64) {
        let mut metrics = self.write_metrics();
        match metrics.get(name) {
            Some(Entry::Counter(counter)) => counter.inc_by(value),
            Some(_) => {
                tracing::warn!(
                    metric = name,
                    "counter increment ignored: name is in use by another metric type"
                );
            }
            None => {
                let Ok(counter) = IntCounter::with_opts(Opts::new(sanitize(name), counter_help()))
                else {
                    tracing::warn!(metric = name, "counter rejected by the registry");
                    return;
                };
                if let Err(e) = self.registry.register(Box::new(counter.clone())) {
                    tracing::warn!(metric = name, error = %e, "counter registration failed");
                    return;
                }
                counter.inc_by(value);
                metrics.insert(name.to_string(), Entry::Counter(counter));
            }
        }
    }

    /// Set a gauge to an absolute value.
    pub fn gauge(&self, name: &str, value: u64) {
        let mut metrics = self.write_metrics();
        match metrics.get(name) {
            Some(Entry::Gauge(gauge)) => gauge.set(value as i64),
            Some(_) => {
                tracing::warn!(
                    metric = name,
                    "gauge set ignored: name is in use by another metric type"
                );
            }
            None => {
                let Ok(gauge) = IntGauge::with_opts(Opts::new(sanitize(name), gauge_help())) else {
                    tracing::warn!(metric = name, "gauge rejected by the registry");
                    return;
                };
                if let Err(e) = self.registry.register(Box::new(gauge.clone())) {
                    tracing::warn!(metric = name, error = %e, "gauge registration failed");
                    return;
                }
                gauge.set(value as i64);
                metrics.insert(name.to_string(), Entry::Gauge(gauge));
            }
        }
    }

    /// Record a single observation in a histogram.
    pub fn histogram(&self, name: &str, value: f64) {
        let mut metrics = self.write_metrics();
        match metrics.get(name) {
            Some(Entry::Histogram(histogram)) => histogram.observe(value),
            Some(_) => {
                tracing::warn!(
                    metric = name,
                    "histogram observation ignored: name is in use by another metric type"
                );
            }
            None => {
                let opts = HistogramOpts::new(sanitize(name), histogram_help())
                    .buckets(DURATION_BUCKETS_MS.to_vec());
                let Ok(histogram) = Histogram::with_opts(opts) else {
                    tracing::warn!(metric = name, "histogram rejected by the registry");
                    return;
                };
                if let Err(e) = self.registry.register(Box::new(histogram.clone())) {
                    tracing::warn!(metric = name, error = %e, "histogram registration failed");
                    return;
                }
                histogram.observe(value);
                metrics.insert(name.to_string(), Entry::Histogram(histogram));
            }
        }
    }

    /// Current value of every counter, by name.
    #[must_use]
    pub fn counters(&self) -> HashMap<String, u64> {
        self.read_metrics()
            .iter()
            .filter_map(|(name, metric)| match metric {
                Entry::Counter(counter) => Some((name.clone(), counter.get())),
                _ => None,
            })
            .collect()
    }

    /// Current value of every gauge, by name.
    #[must_use]
    pub fn gauges(&self) -> HashMap<String, u64> {
        self.read_metrics()
            .iter()
            .filter_map(|(name, metric)| match metric {
                Entry::Gauge(gauge) => Some((name.clone(), gauge.get().max(0) as u64)),
                _ => None,
            })
            .collect()
    }

    /// Summary statistics for every histogram, by name.
    #[must_use]
    pub fn histograms(&self) -> HashMap<String, HistogramStats> {
        self.read_metrics()
            .iter()
            .filter_map(|(name, metric)| match metric {
                Entry::Histogram(histogram) => Some((name.clone(), histogram_stats(histogram))),
                _ => None,
            })
            .collect()
    }

    /// A counter with labels, created on first use.
    ///
    /// The labels are the dimensions the metric is broken down by — a
    /// route, a method, a status. The vector is kept, so the caller can
    /// hold on to it and record against it without a lookup.
    pub fn counter_with_labels(
        &self,
        name: &str,
        help: &str,
        label_names: &[&str],
    ) -> anyhow::Result<IntCounterVec> {
        let mut counters = self
            .label_counters
            .write()
            .unwrap_or_else(|e| e.into_inner());

        if let Some(counter) = counters.get(name) {
            return Ok(counter.clone());
        }

        let counter = IntCounterVec::new(Opts::new(sanitize(name), help), label_names)
            .map_err(|e| anyhow::anyhow!("could not create the counter {name}: {e}"))?;
        self.registry
            .register(Box::new(counter.clone()))
            .map_err(|e| anyhow::anyhow!("could not register the counter {name}: {e}"))?;
        counters.insert(name.to_string(), counter.clone());
        Ok(counter)
    }

    /// A histogram with labels, created on first use.
    pub fn histogram_with_labels(
        &self,
        name: &str,
        help: &str,
        label_names: &[&str],
        buckets: &[f64],
    ) -> anyhow::Result<HistogramVec> {
        let mut histograms = self
            .label_histograms
            .write()
            .unwrap_or_else(|e| e.into_inner());

        if let Some(histogram) = histograms.get(name) {
            return Ok(histogram.clone());
        }

        let histogram = HistogramVec::new(
            HistogramOpts::new(sanitize(name), help).buckets(buckets.to_vec()),
            label_names,
        )
        .map_err(|e| anyhow::anyhow!("could not create the histogram {name}: {e}"))?;
        self.registry
            .register(Box::new(histogram.clone()))
            .map_err(|e| anyhow::anyhow!("could not register the histogram {name}: {e}"))?;
        histograms.insert(name.to_string(), histogram.clone());
        Ok(histogram)
    }

    /// Record one HTTP request.
    ///
    /// The route is a template, not a concrete path: a path with a
    /// parameter in it would otherwise make one label per request.
    pub fn observe_http(
        &self,
        service: &str,
        route: &str,
        method: &str,
        status: u16,
        seconds: f64,
    ) -> anyhow::Result<()> {
        let requests = self.counter_with_labels(
            HTTP_REQUESTS_METRIC,
            "Total HTTP requests, labelled by route template, method and status",
            &["service", "route", "method", "status"],
        )?;
        let duration = self.histogram_with_labels(
            HTTP_DURATION_METRIC,
            "HTTP request duration in seconds",
            &["service", "route", "method"],
            HTTP_DURATION_BUCKETS,
        )?;

        requests
            .with_label_values(&[service, route, method, &status.to_string()])
            .inc();
        duration
            .with_label_values(&[service, route, method])
            .observe(seconds);
        Ok(())
    }

    /// A JSON snapshot of every metric.
    ///
    /// The shape is the one the status endpoint serves, and the one the
    /// existing tests assert on.
    #[must_use]
    pub fn export_json(&self) -> serde_json::Value {
        self.gauge(UPTIME_METRIC, self.uptime_secs());
        serde_json::json!({
            "session_id": crate::session_id(),
            "uptime_secs": self.uptime_secs(),
            "counters": self.counters(),
            "gauges": self.gauges(),
            "histograms": self.histograms(),
        })
    }

    /// The Prometheus text exposition format for every metric.
    #[must_use]
    pub fn render_prometheus(&self) -> String {
        self.gauge(UPTIME_METRIC, self.uptime_secs());
        let families = self.registry.gather();
        TextEncoder::new()
            .encode_to_string(&families)
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "failed to encode metrics for Prometheus");
                String::new()
            })
    }

    fn read_metrics(&self) -> std::sync::RwLockReadGuard<'_, HashMap<String, Entry>> {
        self.metrics.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write_metrics(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<String, Entry>> {
        self.metrics.write().unwrap_or_else(|e| e.into_inner())
    }
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Telemetry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Telemetry")
            .field("uptime_secs", &self.uptime_secs())
            .finish_non_exhaustive()
    }
}

/// Replace every character Prometheus does not accept in a metric name.
///
/// Prometheus allows `[a-zA-Z_:]` followed by `[a-zA-Z0-9_:]*`. Anything
/// else becomes an underscore, so a caller can pass a human-readable name
/// and still get a valid metric.
fn sanitize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (index, ch) in name.chars().enumerate() {
        let valid = if index == 0 {
            ch.is_ascii_alphabetic() || ch == '_' || ch == ':'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'
        };
        out.push(if valid { ch } else { '_' });
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

fn counter_help() -> String {
    "Foodshare tools counter".to_string()
}

fn gauge_help() -> String {
    "Foodshare tools gauge".to_string()
}

fn histogram_help() -> String {
    "Foodshare tools histogram".to_string()
}

/// Summarise a histogram from its bucket distribution.
///
/// The percentiles are interpolated within their bucket, the same way
/// Prometheus' own `histogram_quantile` does. The bounds are the edges of
/// the outermost populated buckets, so `min` and `max` are bounds rather
/// than exact observations.
fn histogram_stats(histogram: &Histogram) -> HistogramStats {
    let collected = histogram.collect();
    let Some(proto) = collected
        .first()
        .and_then(|family| family.get_metric().first())
        .map(|metric| metric.get_histogram())
    else {
        return HistogramStats {
            count: 0,
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
        };
    };

    let count = proto.get_sample_count();
    let sum = proto.get_sample_sum();
    let mean = if count == 0 { 0.0 } else { sum / count as f64 };

    // Cumulative bucket counts, ascending by upper bound.
    let buckets: Vec<(f64, u64)> = proto
        .get_bucket()
        .iter()
        .map(|b| (b.get_upper_bound(), b.get_cumulative_count()))
        .collect();

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut previous_cumulative = 0;
    let mut previous_bound = 0.0;
    for (upper_bound, cumulative) in &buckets {
        if *cumulative > previous_cumulative {
            if min.is_infinite() {
                min = previous_bound;
            }
            max = *upper_bound;
        }
        previous_cumulative = *cumulative;
        previous_bound = *upper_bound;
    }
    if min.is_infinite() {
        min = 0.0;
    }
    if max.is_infinite() {
        max = 0.0;
    }

    HistogramStats {
        count,
        min,
        max,
        mean,
        p50: quantile(&buckets, count, 0.50),
        p95: quantile(&buckets, count, 0.95),
        p99: quantile(&buckets, count, 0.99),
    }
}

/// The `q` quantile of a bucket distribution, interpolated within its
/// bucket.
///
/// Mirrors Prometheus' `histogram_quantile`: the target rank is
/// `q * count`, the bucket that contains it is found, and the value is
/// interpolated between that bucket's upper bound and the previous one.
fn quantile(buckets: &[(f64, u64)], count: u64, q: f64) -> f64 {
    if count == 0 || buckets.is_empty() {
        return 0.0;
    }
    let rank = q * count as f64;
    let mut previous_bound = 0.0;
    let mut previous_cumulative = 0;
    for (upper_bound, cumulative) in buckets {
        if *cumulative as f64 >= rank {
            let span = *cumulative as f64 - previous_cumulative as f64;
            if span <= 0.0 {
                return *upper_bound;
            }
            let within = (rank - previous_cumulative as f64) / span;
            return previous_bound + (upper_bound - previous_bound) * within;
        }
        previous_bound = *upper_bound;
        previous_cumulative = *cumulative;
    }
    // The +Inf bucket: everything landed above the last finite bound.
    previous_bound
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_keeps_valid_names() {
        assert_eq!(sanitize("files_processed"), "files_processed");
        assert_eq!(sanitize("foodshare:builds"), "foodshare:builds");
    }

    #[test]
    fn test_sanitize_replaces_invalid_characters() {
        assert_eq!(sanitize("files processed"), "files_processed");
        assert_eq!(sanitize("a-b.c"), "a_b_c");
        assert_eq!(sanitize("1leading"), "_leading");
    }

    #[test]
    fn test_sanitize_empty_name_is_usable() {
        assert_eq!(sanitize(""), "_");
    }

    #[test]
    fn test_counter_increments() {
        let t = Telemetry::new();
        t.increment("test_counter");
        t.increment("test_counter");
        t.increment_by("test_counter", 3);
        assert_eq!(t.counters().get("test_counter").copied(), Some(5));
    }

    #[test]
    fn test_gauge_sets_absolute_value() {
        let t = Telemetry::new();
        t.gauge("test_gauge", 42);
        t.gauge("test_gauge", 100);
        assert_eq!(t.gauges().get("test_gauge").copied(), Some(100));
    }

    #[test]
    fn test_histogram_records_observations() {
        let t = Telemetry::new();
        t.histogram("test_histogram", 1.0);
        t.histogram("test_histogram", 2.0);
        let stats = t.histograms();
        let stats = stats.get("test_histogram").expect("histogram registered");
        assert_eq!(stats.count, 2);
        assert!((stats.mean - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_type_change_is_ignored() {
        let t = Telemetry::new();
        t.increment("shared_name");
        t.gauge("shared_name", 5);
        // The counter keeps its identity; the gauge request is dropped.
        assert_eq!(t.counters().get("shared_name").copied(), Some(1));
        assert!(t.gauges().get("shared_name").is_none());
    }

    #[test]
    fn test_export_json_shape() {
        let t = Telemetry::new();
        t.increment("jobs_total");
        t.gauge("active_jobs", 2);
        let json = t.export_json();
        assert_eq!(json["counters"]["jobs_total"], 1);
        assert_eq!(json["gauges"]["active_jobs"], 2);
        assert!(json["session_id"].is_string());
        assert!(json["uptime_secs"].is_number());
    }

    #[test]
    fn test_render_prometheus_contains_metric() {
        let t = Telemetry::new();
        t.increment("rendered_total");
        let text = t.render_prometheus();
        assert!(text.contains("rendered_total 1"), "got: {text}");
        assert!(
            text.contains("# TYPE rendered_total counter"),
            "got: {text}"
        );
    }

    #[test]
    fn test_quantile_interpolates() {
        // Two buckets: <=10 holds 5 observations, <=20 holds 10.
        let buckets = vec![(10.0, 5), (20.0, 10)];
        assert!((quantile(&buckets, 10, 0.5) - 10.0).abs() < 1e-9);
        assert!((quantile(&buckets, 10, 1.0) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_quantile_of_empty_is_zero() {
        assert_eq!(quantile(&[], 0, 0.5), 0.0);
        assert_eq!(quantile(&[(1.0, 0)], 0, 0.5), 0.0);
    }
}
