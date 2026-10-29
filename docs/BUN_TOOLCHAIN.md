# Bun toolchain

FoodShare pins stable **Bun 1.4.4**, released October 22, 2026, across the app,
backend, web and tools repositories. `packageManager`, minimum Bun engine,
`@types/bun`, lockfiles, mise and CI use the same release. Backend workspace
packages carry the same minimum and type version.

The local executable must report both `bun --version` and `bun --revision`.
The version alone can hide a canary suffix; stable 1.4.4 reports revision
`1.4.4+38afee6c8`. For a self-installed canary, `bun upgrade --stable` switches
to the stable release. Use `mise install` for the native `bun = "1.4.4"` pin.
The previous npm-backed canary bootstrap is no longer required.

CI installs exact 1.4.4. Web container stages use the release and slim images
with immutable multi-architecture digests. Do not substitute a rolling canary
tag while retaining a stable version claim.

Use Bun for installation, scripts and native `bun fmt`; use TSRS for TypeScript
static checks. Oxlint and Biome packages, configs and executable callers have
been retired at the owner’s request. Stable 1.4.4 does not implement `bun lint`
in the verified binary, so no unavailable linter command is substituted. TSRS
does not reproduce every former linter rule. Domain, security, RTL and test
checks remain separate requirements. Deno lint/format still cover Edge Functions.

Formatting adoption checks changed JavaScript, TypeScript, JSON and CSS files.
CI uses the committed push/PR range, not an empty clean-worktree comparison.
Generated workspace files and vendored/third-party code are excluded. Git hooks
use native staged formatting; partially staged files are protected by Bun.
The current check is carried in each repository for independent CI execution;
its canonical source and behavioral tests live in foodshare-tools.

Regenerate locks with Bun and verify frozen installs. Preserve unrelated source
and dependency edits. Report local validation, remote CI, runner-image rollout
and running production image revisions separately. This upgrade does not resolve
FoodShare's existing database-test, secret-scan, backend-deployment or store gates.

Sources: [Bun 1.4.4 release notes](https://bun.com/blog/release-notes/bun-v1.4.4)
and [mise Bun support](https://mise.jdx.dev/lang/bun.html).
