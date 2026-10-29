# Workspace Rules (`foodshare-tools`)

- **Core Toolchain Mandate (Bun, TSRS):** This repo owns the shared
  toolchain (lefthook hooks, CLIs, simulators, builders).
  - **TSRS:** Use the pinned TSRS compiler for TypeScript static checks.
  - **Formatting:** Use Bun 1.4.4 native `bun fmt`; removed lint/format packages must not be reintroduced.
  - **Bun:** For all package management and script execution tasks, ALWAYS use `bun` instead of `npm`, `npx`, `yarn`, or `pnpm`.
- **No container runtime:** Nothing in this repository needs Docker, Podman, or
  any other container runtime — no image is built, no container is run, no
  container socket is opened. Do not add a dependency on one, and do not
  shell out to one. A tool that must reach a container has to find the
  runtime at run time and report which one it found, rather than assume a
  particular one: assuming is what makes a tool work on one machine and
  fail on the next.

- **Domain rules live with their repos:** `foodshare-web/.agents/AGENTS.md`,
  `foodshare-backend/.agents/AGENTS.md`, `foodshare-app/.agents/AGENTS.md`.
  Keep this file to toolchain concerns only.
