# foodshare-orchestrator

Workspace build, test and verification orchestration.

The orchestrator runs the workspace's five verification domains and keeps
the state of the run as it goes. It is a **library**, not a service, so
the same implementation can be driven from the command line, from a TUI,
or from HTTP.

## Domains

| Domain   | What it checks                                            |
| -------- | --------------------------------------------------------- |
| `rust`   | `cargo test --workspace`                                   |
| `wasm`   | The five WASM bindings built with wasm-pack                |
| `web`    | The Next.js type check and build                           |
| `backend`| The Supabase edge function tests                           |
| `mobile` | The Maestro flow syntax check and the Gradle unit tests    |

## Usage

```rust,no_run
use foodshare_orchestrator::{DomainKind, Orchestrator, Workspace};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let workspace = Workspace::discover()?;
let orchestrator = Orchestrator::new(workspace);

// One domain.
let outcome = orchestrator.run_domain(DomainKind::Rust).await?;
println!("rust took {}ms", outcome.duration_ms);

// Or the whole plan, stopping at the first failure.
let outcome = orchestrator.run_plan().await?;
println!("all green: {}", outcome.success);
# Ok(())
# }
```

## State

The orchestrator keeps the state of every domain, and hands out snapshots
of it. A snapshot is what the TUI renders and what the status endpoint
serves.

```rust,no_run
use foodshare_orchestrator::{DomainKind, Orchestrator, Workspace};

# fn run(orchestrator: Orchestrator) {
let snapshot = orchestrator.snapshot();
if let Some(domain) = snapshot.domain(DomainKind::Web) {
    println!("{} — {:?}", domain.status, domain.logs);
}
# }
```

## Retries and disk healing

A failing step is retried, and the disk is healed between attempts: a
build that runs out of space is the most common reason for a failure that
goes away on its own.

```rust,no_run
use foodshare_orchestrator::disk;

# async fn run(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
let report = disk::heal(root, disk::MIN_FREE_GB, false).await?;
for action in &report.actions {
    println!("{action}");
}
# Ok(())
# }
```

## Workspace discovery

The workspace root is the directory that holds `foodshare-app`. It is
found from the current directory, or set explicitly with
`FOODSHARE_WORKSPACE_ROOT`.

```rust,no_run
use foodshare_orchestrator::Workspace;

# fn run() {
let workspace = Workspace::discover().expect("workspace");
println!("{}", workspace.root().display());
println!("{}", workspace.web().display());
# }
```

## Front-ends

| Front-end                    | How it drives the orchestrator              |
| ---------------------------- | ------------------------------------------- |
| `fs-auto`                    | The headless run-until-green command        |
| `fs-tui`                     | The live monitor, over the same state       |
| `fs-orchestrator serve`      | The HTTP daemon                             |

The library itself has no HTTP dependency, so the CLI and the TUI do not
pay for it.
