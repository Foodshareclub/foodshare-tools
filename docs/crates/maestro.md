# foodshare-maestro

Maestro E2E flow discovery, syntax checking, and multi-device matrix
execution.

This crate replaces the Bun runner that used to drive Maestro. It covers
the three things that runner did: finding the flows in a suite, checking
their syntax, and running them — on one device or on every device at
once.

## Devices

Maestro drives an iOS simulator or an Android emulator. Both are
discovered from the tools that own them, and a tool that is not
installed is not an error: it just means that platform has no devices.

```rust,no_run
use foodshare_maestro::device;

for target in device::discover() {
    println!("[{}] {} ({})", target.platform.name(), target.name, target.id);
}
```

## Flows

A suite is a directory of YAML flows, with optional subflows in a
`subflows` directory. The configuration files are not flows, so they are
left out.

```rust,no_run
use foodshare_maestro::flows;

let found = flows::discover(std::path::Path::new("foodshare-app/.maestro"));
```

## Running

```rust,no_run
use foodshare_maestro::{device, runner};

// Check every flow's syntax.
let results = runner::check_syntax(&found).await;

// Run the suite on one device.
if let Some(target) = device::resolve(None) {
    let outcome = runner::run_test(&target, &[], maestro_dir, None).await;
}

// Or on every device at once, each with its own log.
let outcomes = runner::run_matrix(&devices, &[], maestro_dir, reports_dir).await;
```

The matrix writes one log per device under `.maestro/reports/`, so a
failure on one device does not hide what happened on another.
