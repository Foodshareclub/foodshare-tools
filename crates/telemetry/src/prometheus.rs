//! Prometheus rendering.
//!
//! The registry itself lives in [`crate::metrics`]. This module renders
//! it, and is what an HTTP handler serves on `/metrics`.
//!
//! The crate does not open a port. A service that wants to expose metrics
//! serves [`render`] on its own route, so one process owns one port rather
//! than two servers racing for the same metrics.

use prometheus::proto::MetricFamily;

/// The Prometheus text exposition format for every recorded metric.
#[must_use]
pub fn render() -> String {
    crate::metrics().render_prometheus()
}

/// The registry's metric families, for callers that want the raw protobuf
/// rather than the text format.
#[must_use]
pub fn gather() -> Vec<MetricFamily> {
    crate::metrics().registry().gather()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_contains_recorded_metric() {
        crate::metrics().increment("prometheus_render_total");
        let text = render();
        assert!(text.contains("prometheus_render_total"), "got: {text}");
    }

    #[test]
    fn test_gather_returns_families() {
        crate::metrics().increment("prometheus_gather_total");
        let families = gather();
        assert!(
            families
                .iter()
                .any(|f| f.get_name() == "prometheus_gather_total")
        );
    }
}
