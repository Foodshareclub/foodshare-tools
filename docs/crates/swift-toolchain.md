# foodshare-swift-toolchain

Swift toolchain version management for the Foodshare monorepo.

The iOS app and the other Swift packages each declare a Swift version
in their `Package.swift`. When those declarations drift apart, a build
works on one machine and fails on another, and the reason is a version
number in a file nobody remembers writing.

This crate keeps them in step.

## What it does

| Module | What it answers |
| --- | --- |
| `detect` | Which Swift versions are installed, and which one is in use |
| `verify` | Do the `Package.swift` files agree, and does the machine have what they need |
| `config` | Where the versions are declared, and what the expected one is |
| `migrate` | How to move from one version to another |

## Usage

```rust,no_run
use foodshare_swift_toolchain::{detect::SwiftToolchain, verify};

let installed = SwiftToolchain::detect();
let report = verify::verify_consistency(".").await?;
if !report.is_consistent() {
    println!("{}", report.describe());
}
```

The `foodshare-swift` binary is the command-line face of this crate:

```bash
foodshare-swift --project-root . --format text <subcommand>
```

## The error codes

`SwiftError` distinguishes the failures that need different answers:
a version that is not installed is not the same as two manifests that
disagree with each other, and neither is the same as a toolchain that
is installed but too old.
