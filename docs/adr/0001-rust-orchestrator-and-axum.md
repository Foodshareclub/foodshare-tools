# 0001 — One orchestrator, in Rust, with axum only at the edge

- **Status**: Accepted
- **Date**: 2026-10-20
- **Scope**: `foodshare-tools`

## Context

The workspace's verification — the Rust tests, the WASM build, the web
type check and build, the backend edge function tests, the mobile flow
check and the Gradle unit tests — was written twice.

`cli.ts` owned it in TypeScript, with the retry policy, the disk healing
and the artefact cleanup. `fs-auto` owned it in Rust, with a second copy
of the same policy. `fs-tui`, the Rust live monitor, owned neither: it
simulated its results, and its non-interactive mode shelled out to
`bun cli.ts verify`.

The observability story had the same shape. `foodshare-telemetry`
declared a `metrics` dependency it never used, kept a hand-rolled
registry, and shipped documentation for a `prometheus` module and a
`metrics::counter().inc()` API that did not exist. No crate in the
workspace depended on it. `foodshare-core`'s `HealthChecker` was
implemented and exported, and no binary called it.

The question this record answers is narrow: **when the orchestration
moves to Rust, does the tools repository adopt axum?**

## Decision

**Yes, for the daemon, and only for the daemon.**

1. The orchestration becomes `foodshare-orchestrator`, a library.
2. Three front-ends drive it: `fs-auto`, `fs-tui`, and `fs-orchestrator
   serve`.
3. `fs-orchestrator serve` is the only crate in the repository that
   depends on axum, tower or tower-http.
4. The orchestrator library has no HTTP dependency, so the CLI and the
   TUI never pay for a web framework they do not use.

### Why axum

- **It is already the house framework.** `foodshare-backend/services`
  runs axum 0.8.9 with tower-http (`TraceLayer`, `CatchPanicLayer`,
  `TimeoutLayer`, request ids). Adopting it here means one framework,
  one middleware stack and one set of habits across the workspace, and
  it keeps a future shared crate realistic.
- **The probe contract already exists.** The backend's `job-worker` serves
  `/healthz`, `/readyz` and `/metrics`, and Prometheus already scrapes
  `worker:8080/metrics`. The daemon serves the same routes with the same
  shapes, so one Prometheus and one set of Quadlet health checks cover
  both services.
- **Tower is the ecosystem.** Timeouts, tracing, request ids and panic
  catching are layers, not hand-written code, and they are the same
  layers the backend uses.

### Why not the alternatives

| Option        | Why not                                                                 |
| ------------- | ----------------------------------------------------------------------- |
| `actix-web`   | A separate ecosystem: no tower middleware, so the parity with the backend's `fs-core` telemetry is lost. |
| `warp`        | The filter model does not fit the extractor-based handlers the backend already uses, and tower-http does not apply. |
| `tiny_http`   | Synchronous, and no async runtime: the orchestrator is already tokio-based. |
| `hyper` alone | A routing layer would have to be written, and it would be the fourth framework in the workspace. |
| No HTTP       | Rejected for the daemon, kept for the library. The TUI and the CLI read the orchestrator's state in process, and that is the right shape for them. |

### The panic profile

`[profile.release]` sets `panic = "abort"`, which the backend's services
workspace does not. Under `abort`, a panicking handler takes the whole
process down, and `tower-http`'s `CatchPanicLayer` cannot catch it. A
daemon that serves for the life of the process should not abort on one
bad request.

The release profile is left alone, so the CLI binaries keep their
current size and behaviour, and the daemon gets an inheriting profile:

```toml
[profile.release-service]
inherits = "release"
panic = "unwind"
```

The alternative — dropping `panic = "abort"` for the whole workspace —
is one line and would match the backend, but it changes every shipped
binary to get one of them right.

## What stays in TypeScript

`scripts/check-no-node.ts` stays. It lints JavaScript and TypeScript
files for a runtime policy, and a Rust port of it buys nothing: the guard
is most legible where the policy lives, next to the Bun configuration it
enforces.

The rest of the TypeScript in this repository has been ported. What is
left is a set of shims, one per retired script, that call the Rust
binary behind it:

| TypeScript          | Rust replacement                                |
| ------------------- | ----------------------------------------------- |
| `cli.ts auto`       | `foodshare-orchestrator`, driven by `fs-auto`     |
| `cli.ts verify`     | `foodshare-orchestrator::verify`, driven by `fs-verify` |
| `cli.ts node-check` | `foodshare-orchestrator::verify`                 |
| `cli.ts wasm`       | `foodshare-wasm`                                 |
| `clean-disk.ts`     | `foodshare-orchestrator::disk`                    |
| `maestro-runner.ts` | `foodshare-maestro`                              |
| `maestro-matrix.ts` | `foodshare-maestro`                              |
| `build-wasm.ts`     | `foodshare-wasm`                                 |
| `builder.ts`        | `fs-app::build`                                  |
| `sim-runner.ts`     | `fs-app::sim_flow`                               |
| `xccloud.ts`        | `foodshare-xccloud`                              |

The shims exist because the sibling repositories call these scripts
through the `tools/` symlink. They are one line each, and they go when
the sibling repositories are updated.

## Consequences

- One implementation of the verification, so the CLI, the TUI and the
  daemon cannot drift apart.
- `foodshare-core`'s `HealthChecker` and `foodshare-telemetry` finally
  have consumers.
- The tools repository gains a dependency on axum, tower and tower-http,
  at the same versions the backend already resolves.
- The daemon adds a release profile, so the release workflow needs to
  know about it.
- The Bun orchestrator still drives the web and backend repositories'
  own commands — `bun run type-check`, `deno test` — because those
  repositories are not Rust. That is a boundary of the workspace, not a
  gap in the tools.
- The Xcode Cloud tool needed ES256 signing, so the workspace gained
  `p256`, at the same version the backend's `fs-verify` resolves.


