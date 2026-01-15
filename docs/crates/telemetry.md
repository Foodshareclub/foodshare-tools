# foodshare-telemetry

Observability infrastructure for logging, metrics, and tracing.

## Features

- Structured logging with tracing
- Metrics collection
- Prometheus export
- JSON log format support

## Usage

```rust
use foodshare_telemetry::{init, metrics};
use tracing::{info, warn, error};

// Initialize telemetry
init(&Config::default())?;

// Structured logging
info!(file = "main.rs", line = 42, "Processing file");
warn!(duration_ms = 150, "Slow operation");
error!(code = "E2001", "File not found");

// Metrics
metrics::counter("files_processed").inc();
metrics::histogram("processing_time").record(150.0);
```

## Modules

### `logging`

Structured logging setup.

```rust
use foodshare_telemetry::logging;

// Initialize with defaults
logging::init()?;

// Custom configuration
logging::init_with_config(&LogConfig {
    level: "debug",
    format: LogFormat::Json,
    file: Some("app.log"),
})?;
```

### `metrics`

Metrics collection.

```rust
use foodshare_telemetry::metrics;

// Counter
metrics::counter("operations_total").inc();
metrics::counter("errors_total").inc_by(5);

// Histogram
metrics::histogram("duration_seconds").record(0.15);

// Gauge
metrics::gauge("active_connections").set(42.0);
```

### `prometheus`

Prometheus export.

```rust
use foodshare_telemetry::prometheus;

// Get metrics as Prometheus format
let output = prometheus::export()?;

// Start metrics server
prometheus::serve(9090)?;
```

## Configuration

```toml
[telemetry]
enabled = true
log_level = "info"
log_format = "json"  # or "pretty"
metrics_endpoint = "http://localhost:9090"
```

## Log Levels

| Level | Usage |
|-------|-------|
| `trace` | Very detailed debugging |
| `debug` | Debugging information |
| `info` | General information |
| `warn` | Warning conditions |
| `error` | Error conditions |
