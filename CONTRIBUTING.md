# Contributing to FoodShare foodshare-tools

FoodShare is a small, hands-on team. Development is driven by the maintainers and their
agent fleet; external contributions are welcome as issues and discussion.

## Before you start

1. Read [GEMINI.md](GEMINI.md). It is the authoritative description of the toolchain
   and the invariants of this codebase — the rules in it exist because something broke
   before.
2. Search existing issues before opening a new one.
3. Feature and bug intake goes through the **agent-intake** issue form in the Issues
   tab; the maintainers route from there.

## Issues and labels

| Label          | Meaning                                                        |
| -------------- | -------------------------------------------------------------- |
| `intake`       | Filed through the agent-intake form, awaiting triage           |
| `p0`           | Priority 0 — blocks production, handled first                  |
| `prod-gate`    | Work that must land before a production release                |
| `security`     | Security-relevant; see [SECURITY.md](SECURITY.md) for reports  |

## Branching and delivery

`main` is the only long-lived branch. Work is delivered to `origin/main` once the CI
gates pass; a push to `main` runs the deployment pipeline, so a commit is not "done"
until its CI is green.

## Toolchain

Rust crates are checked with `cargo fmt --check`, `cargo clippy` and `cargo test`; TypeScript runs on Bun.

## Conduct

By participating in this project you agree to abide by the
[Code of Conduct](CODE_OF_CONDUCT.md).
