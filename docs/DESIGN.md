# Foodshare Tools — what this repository is

This file used to hold a phase plan for a Sentry-instrumented Next.js
application. That application is not in this repository, and the plan
described a toolchain this repository does not use: `npx`, `setup-node`,
ESLint, Vitest and a `turbo.json` with tasks that do not exist. It was
kept because plans are kept, and it was wrong because nobody read it
again.

What is here instead is the short version of what this repository is
for and where the decisions are written down.

## What this repository is

The toolchain the other three repositories of the estate share: the git
hooks, the secret scanner, the commit-message check, the build and
verification orchestrator, and the small libraries the rest of the estate
publishes.

It is Rust and Bun. There is no Node.js in the toolchain, and the build
fails if one creeps back in — see [the guard](scripts/check-no-node.ts)
and [the toolchain section of the README](README.md#toolchain).

## Where the decisions are written down

- [ADR 0001 — the Rust orchestrator and axum](docs/adr/0001-rust-orchestrator-and-axum.md)
  is why the verification is a daemon in Rust rather than a script in
  Bun, and what the probe contract is.
- [docs/architecture.md](docs/architecture.md) is how the pieces fit
  together.
- [docs/cli-reference.md](docs/cli-reference.md) is every command and
  flag, generated from the binaries' own help.
- [docs/configuration.md](docs/configuration.md) is what a
  `.foodshare-hooks.toml` may say, and which of it this build reads.

## The rules the tools follow

These are the rules the work is judged against, and they are the reason
several of the tools are shaped the way they are.

**A tool that cannot be run is not a tool.** The estate's tools run on
more than one machine, so a tool finds what it needs at run time and
reports what it found, rather than assuming a particular one. Assuming
is how a tool ends up working on one machine and failing on the next.

**A guard that reports a problem and then succeeds reports and does
nothing.** Every check that can fail has an exit code that says it
failed, and the exit codes say *which* failure it was: a secret that
leaked is 5, a message that did not pass the format is 2, a scan that
could not look at all is 4. A pipeline can tell those apart.

**A configuration that is written and read nowhere is worse than one
that is not written.** The loader reports the keys it does not
implement, naming each one. A repository that writes a section the tool
does not read learns that the section does nothing.

**There is one implementation of each thing.** A second copy of a
function, a type or a rule is a second thing to keep in step with the
first, and it will drift.

**The documentation is generated from the code where the code can
answer.** The CLI reference is the binaries' own help; the error codes
are the ones the error type defines; the crate documents are the
exports.
