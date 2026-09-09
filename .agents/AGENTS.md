# Workspace Rules (`foodshare-tools`)

- **Core Toolchain Mandate (Bun, Oxlint, Biome):** This repo owns the shared
  toolchain (lefthook hooks, CLIs, simulators, builders).
  - **Bun:** For all package management and script execution tasks, ALWAYS use `bun` instead of `npm`, `npx`, `yarn`, or `pnpm`.
  - **Oxlint:** Enforce Rust-based fast linting (`oxlint` / `bunx oxlint`) across JS/TS/JSON code.
  - **Biome:** Enforce Rust-based fast formatting & linting (`biome` / `bunx biome check`).

- **Domain rules live with their repos:** `foodshare-web/.agents/AGENTS.md`,
  `foodshare-backend/.agents/AGENTS.md`, `foodshare-app/.agents/AGENTS.md`.
  Keep this file to toolchain concerns only.
