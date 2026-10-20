# foodshare-telemetry

Observability infrastructure for logging, metrics, and tracing.

## Features

- Structured logging with tracing
- Metrics collection, backed by a Prometheus registry
- Prometheus text exposition, for scraping
- JSON export, for the status endpoint and for tests

The registry is the single source of truth. It renders to the Prometheus
text exposition format for scraping, and to JSON for the status endpoint.

## Usage

```rust
use foodshare_telemetry::{metrics, prometheus};

metrics().increment("files_processed");
metrics().increment_by("files_processed", 3);
metrics().gauge("active_jobs", 2);
metrics().histogram("processing_ms", 12.5);

// The Prometheus text exposition format.
let text = prometheus::render();
assert!(text.contains("files_processed"));
```

## Modules

### `metrics`

The registry.

```rust
use foodshare_telemetry::metrics;

// Counter
metrics().increment("operations_total");
metrics().increment_by("operations_total", 5);

// Histogram
metrics().histogram("duration_ms", 0.15);

// Gauge
metrics().gauge("active_connections", 42);
```

Metric names are sanitised to the character set Prometheus accepts, so a
caller can pass a human-readable name and still get a valid metric. A
name is registered on first use, and a name already in use by a different
metric type is left alone.

### `prometheus`

Rendering.

```rust
use foodshare_telemetry::prometheus;

// The Prometheus text exposition format.
let output = prometheus::render();

// The raw metric families, for callers that want the protobuf.
let families = prometheus::gather();
```

The crate does not open a port. A service that wants to expose metrics
serves `prometheus::render()` on its own route, so one process owns one
port rather than two servers racing for the same metrics. The
`fs-orchestrator` daemon does exactly that, on `/metrics`.

### Logging

```rust
use foodshare_telemetry::{init, init_with_config, TelemetryConfig};

init()?;
init_with_config(TelemetryConfig {
    log_level: "debug".to_string(),
    ..TelemetryConfig::default()
});
```

### Timers

```rust
use foodshare_telemetry::Timer;

let timer = Timer::start("scanning");
let duration = timer.stop();
```

The duration is recorded in the `scanning` histogram, in milliseconds.

## Configuration

```toml
[telemetry]
enabled = true
log_level = "info"
log_format = "json"  # or "pretty"
metrics_endpoint = "http://localhost:9090"
```

## Log Levels

| Level  | Usage                          |
| ------ | ------------------------------ |
| `trace` | Very detailed debugging        |
| `debug` | Debugging information          |
| `info`  | General information            |
| `warn`  | Warning conditions             |
| `error` | Error conditions               |
