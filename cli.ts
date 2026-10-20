#!/usr/bin/env bun
/**
 * FoodShare Unified 10x Developer CLI & Orchestrator
 *
 * Single command-line interface for the entire FoodShare mono-ecosystem:
 * - Mobile: iOS Simulator & Android Emulator management, Skip build, Maestro E2E
 * - Web: Next.js 16 App Router Turbopack builds, Biome/Oxlint validation
 * - Backend: Supabase Edge Functions, pgvector migrations, Deno tests
 * - Infra/Storage: Proactive disk health monitoring and cache auto-healing
 *
 * The workspace orchestration itself lives in Rust
 * (`foodshare-orchestrator`); this file is the front-end that dispatches
 * to it and keeps the per-repository commands that are not Rust yet.
 *
 * Usage:
 *   bun cli.ts auto [--keep] [--retry N] [--clean] [--quick]  (run-until-green, build, auto-close & clean)
 *   bun cli.ts orchestrate [serve|run|status]  (the Rust orchestrator)
 *   bun cli.ts mobile [run|smoke|matrix|build|test|clean|syntax|hierarchy]
 *   bun cli.ts web [build|dev|lint|typecheck]
 *   bun cli.ts backend [sync-types|test]
 *   bun cli.ts tui [verify|rust|wasm|web|backend|mobile]
 *   bun cli.ts verify
 *   bun cli.ts node-check
 *   bun cli.ts disk:heal [--force]
 *   bun cli.ts check
 *
 * @module tools/cli
 */

import { existsSync } from "bun:fs";
import { join } from "bun:path";

function resolveWorkspaceRoot(): string {
  if (existsSync(join(process.cwd(), "foodshare-app"))) {
    return process.cwd();
  }
  if (existsSync(join(process.cwd(), "..", "foodshare-app"))) {
    return join(process.cwd(), "..");
  }
  return process.cwd();
}

const ROOT_DIR = resolveWorkspaceRoot();
const APP_DIR = join(ROOT_DIR, "foodshare-app");
const WEB_DIR = join(ROOT_DIR, "foodshare-web");
const BACKEND_DIR = join(ROOT_DIR, "foodshare-backend");
const TOOLS_DIR = join(ROOT_DIR, "foodshare-tools");

/**
 * Every repository in the workspace, with the directory its guard lives in.
 *
 * The Node.js guard is per-repository — each one knows its own layout, its
 * own Deno carve-out, and its own container files — but the *policy* is
 * one: Bun is the only runtime. This runs all four so the policy is
 * enforced across the workspace, not just inside whichever repository you
 * happen to be standing in.
 */
const NODE_GUARD_REPOS = [
  { name: "foodshare-web", dir: WEB_DIR },
  { name: "foodshare-app", dir: APP_DIR },
  { name: "foodshare-backend", dir: BACKEND_DIR },
  { name: "foodshare-tools", dir: TOOLS_DIR },
];

/**
 * Run the Node.js guard in every repository in the workspace.
 *
 * Returns the number of repositories that failed, so the caller can decide
 * what to do about it.
 */
async function runNodeGuards(): Promise<number> {
  let failed = 0;

  for (const repo of NODE_GUARD_REPOS) {
    const script = join(repo.dir, "scripts", "check-no-node.ts");
    if (!existsSync(script)) {
      console.error(`❌ ${repo.name}: scripts/check-no-node.ts is missing`);
      failed += 1;
      continue;
    }

    console.log(`\n🛡️  [${repo.name}] Node.js guard...`);
    const code = await runCommand(["bun", "scripts/check-no-node.ts"], repo.dir);
    if (code !== 0) {
      console.error(`❌ ${repo.name}: Node.js guard failed.`);
      failed += 1;
    }
  }

  return failed;
}

async function runCommand(
  cmd: string[],
  cwd: string = ROOT_DIR,
): Promise<number> {
  console.log(`⚡ [${cwd.split("/").pop()}] Running: ${cmd.join(" ")}`);
  const proc = Bun.spawn(cmd, {
    cwd,
    stdout: "inherit",
    stderr: "inherit",
  });
  return await proc.exited;
}

async function handleMobile(args: string[]) {
  const action = args[0] || "run";
  const subArgs = args.slice(1);

  switch (action) {
    case "run":
      await runRustBin("fs-sim");
      break;
    case "smoke":
      await runRustBin("fs-sim", ["--smoke"]);
      break;
    case "matrix":
      await runRustBin("fs-maestro", ["matrix", ...subArgs]);
      break;
    case "build":
      await runRustBin("fs-build", subArgs);
      break;
    case "test":
      await runRustBin("fs-sim", ["--test"]);
      break;
    case "clean":
      await runRustBin("fs-sim", ["--clean"]);
      break;
    case "syntax":
      await runRustBin("fs-maestro", ["syntax"]);
      break;
    default:
      // Forward directly to the Maestro runner
      await runRustBin("fs-maestro", args);
      break;
  }
}

async function handleWeb(args: string[]) {
  const action = args[0] || "build";
  const subArgs = args.slice(1);
  switch (action) {
    case "build":
      await runCommand(["bun", "run", "build"], WEB_DIR);
      break;
    case "dev":
      await runCommand(["bun", "run", "dev"], WEB_DIR);
      break;
    case "lint":
      await runCommand(["bun", "run", "lint"], WEB_DIR);
      break;
    case "typecheck":
      await runCommand(["bun", "run", "type-check"], WEB_DIR);
      break;
    case "translations":
    case "i18n":
    case "sync-translations":
      await runCommand(
        ["bun", "run", "translations:sync", ...subArgs],
        WEB_DIR,
      );
      break;
    default:
      await runCommand(["bun", "run", ...args], WEB_DIR);
      break;
  }
}

async function handleBackend(args: string[]) {
  const action = args[0] || "test";
  switch (action) {
    case "test":
      await runCommand(
        [
          "deno",
          "test",
          "--allow-all",
          "--config",
          "supabase/functions/deno.json",
          "supabase/functions/__tests__/",
        ],
        BACKEND_DIR,
      );
      break;
    case "sync-types":
      await runCommand(
        ["bunx", "--bun", "supabase", "gen", "types", "typescript", "--local"],
        BACKEND_DIR,
      );
      break;
    default:
      await runCommand(["bun", "run", ...args], BACKEND_DIR);
      break;
  }
}

/**
 * Build a Rust binary if it is missing, then run it.
 *
 * The Rust binaries own the workspace orchestration. This is the shim that
 * keeps `bun cli.ts` working while the TypeScript front-ends are retired
 * one at a time.
 */
async function runRustBin(name: string, args: string[] = []) {
  const bin = join(ROOT_DIR, "foodshare-tools", "target", "debug", name);
  if (!existsSync(bin)) {
    console.log(`🔧 Building ${name} (Rust)...`);
    const code = await runCommand(
      ["cargo", "build", "--bin", name],
      join(ROOT_DIR, "foodshare-tools"),
    );
    if (code !== 0) {
      console.error(`❌ Failed to build ${name}.`);
      process.exit(code);
    }
  }
  console.log(`🦀 ${name} ${args.join(" ")}`);
  const proc = Bun.spawn([bin, ...args], {
    cwd: ROOT_DIR,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  const code = await proc.exited;
  if (code !== 0) {
    process.exit(code);
  }
}

/**
 * Prune the caches and report the free space.
 *
 * The disk healing lives in Rust, with the rest of the orchestrator.
 */
async function runDiskHeal(force: boolean) {
  const args = ["disk"];
  if (force) args.push("--force");
  await runRustBin("fs-orchestrator", args);
}

async function handleTui(args: string[]) {
  // Pass through args: verify/rust/wasm/web/backend/mobile or interactive if no args
  await runRustBin("fs-tui", args);
}

// ── Auto Orchestrator: run-until-green, build, auto-close & clean ──────────
//
// The orchestration lives in Rust now (`foodshare-orchestrator`), so this
// file only dispatches to it. The Bun implementation that used to live
// here duplicated the retry policy, the disk healing and the artefact
// cleanup that the Rust orchestrator owns.


async function runNodeCheck() {
  console.log("🛡️ =========================================================");
  console.log("🛡️  FoodShare Node.js Guard — every repository");
  console.log("🛡️ =========================================================");

  const failed = await runNodeGuards();

  if (failed > 0) {
    console.error(
      `\n❌ ${failed} repository/repositories let Node.js back into the toolchain.`,
    );
    process.exit(1);
  }

  console.log("\n✅ Every repository runs on Bun. No Node.js in the toolchain.");
}

async function runFullVerification() {
  console.log("🌟 =========================================================");
  console.log("🌟 FoodShare 10x Full Domain Verification");
  console.log("🌟 =========================================================");

  // 0. The toolchain policy, across every repository in the workspace.
  //    Bun is the only runtime; this fails the whole verification if any
  //    repository has let Node.js back in.
  console.log("\n🛡️  [0/5] Running the Node.js guard in every repository...");
  const nodeFailures = await runNodeGuards();
  if (nodeFailures > 0) {
    console.error(
      `\n❌ Node.js guard failed in ${nodeFailures} repository/repositories.`,
    );
    process.exit(1);
  }

  // 1. Disk Health
  await runDiskHeal(false);

  // 2. Rust Crates & Test Engine
  console.log("\n🦀 [1/5] Running Rust 2024 Workspace Test Engine...");
  const rustCode = await runCommand(
    ["cargo", "test", "--workspace"],
    join(ROOT_DIR, "foodshare-tools"),
  );
  if (rustCode !== 0) {
    console.error("❌ Rust workspace tests failed.");
    process.exit(1);
  }

  // 3. Web Build & Type Check
  console.log("\n📦 [2/5] Building Web Application with Next.js Turbopack...");
  const webBuildCode = await runCommand(["bun", "run", "build"], WEB_DIR);
  if (webBuildCode !== 0) {
    console.error("❌ Web build failed.");
    process.exit(1);
  }

  // 4. Mobile Smoke Test Suite
  console.log("\n📱 [3/5] Executing Mobile Simulator Runner & Smoke Tests...");
  await runRustBin("fs-sim", ["--smoke"]);

  // 5. Backend Edge Function Deno Unit Tests
  console.log(
    "\n⚡ [4/5] Executing Backend Supabase Edge Function Deno Tests...",
  );
  const backendCode = await runCommand(
    [
      "deno",
      "test",
      "--allow-all",
      "--config",
      "supabase/functions/deno.json",
      "supabase/functions/__tests__/",
    ],
    BACKEND_DIR,
  );
  if (backendCode !== 0) {
    console.error("❌ Backend edge function tests failed.");
    process.exit(1);
  }

  console.log("\n==========================================");
  console.log("🎉 ALL DOMAIN SYSTEMS VERIFIED 100% GREEN!");
  console.log("==========================================\n");
}

async function printStatus() {
  console.log("🌟 =========================================================");
  console.log("🌟 FoodShare 10x Unified Workspace Health");
  console.log("🌟 =========================================================");
  await runDiskHeal(false);

  console.log("\n📦 Monorepo Workspaces:");
  console.log(
    `  • FoodShare App:     ${existsSync(APP_DIR) ? "✅ Ready" : "❌ Missing"}`,
  );
  console.log(
    `  • FoodShare Web:     ${existsSync(WEB_DIR) ? "✅ Ready" : "❌ Missing"}`,
  );
  console.log(
    `  • FoodShare Backend: ${existsSync(BACKEND_DIR) ? "✅ Ready" : "❌ Missing"}`,
  );
}

async function main() {
  const args = process.argv.slice(2);
  const domain = args[0] || "check";
  const subArgs = args.slice(1);

  switch (domain) {
    case "auto":
      await runRustBin("fs-auto", subArgs);
      break;
    case "orchestrate":
      // The HTTP daemon and the orchestrator CLI: serve, run, status.
      await runRustBin("fs-orchestrator", subArgs);
      break;
    case "mobile":
      await handleMobile(subArgs);
      break;
    case "web":
      await handleWeb(subArgs);
      break;
    case "backend":
      await handleBackend(subArgs);
      break;
    case "tui":
      await handleTui(subArgs);
      break;
    case "rust":
      await runCommand(
        ["cargo", "test", "--workspace", ...subArgs],
        join(ROOT_DIR, "foodshare-tools"),
      );
      break;
    case "wasm":
      await runRustBin("fs-wasm", subArgs);
      break;
    case "bench":
      await runCommand(
        [
          "cargo",
          "bench",
          "--bench",
          "vector_benchmark",
          "--bench",
          "crypto_benchmark",
          "--bench",
          "distance",
          "--bench",
          "compress",
          "--bench",
          "secrets_bench",
          "--bench",
          "resize",
          ...subArgs,
        ],
        join(ROOT_DIR, "foodshare-tools"),
      );
      break;
    case "translations":
    case "i18n":
      await runCommand(
        ["bun", "run", "translations:sync", ...subArgs],
        WEB_DIR,
      );
      break;
    case "verify":
      await runFullVerification();
      break;
    case "node-check":
      await runNodeCheck();
      break;
    case "disk:heal":
      await runDiskHeal(subArgs.includes("--force"));
      break;
    case "check":
    case "status":
      await printStatus();
      break;
    default:
      console.log(`
Usage:
  bun cli.ts auto [--keep] [--retry N] [--clean] [--quick] [--sequential]  (run-until-green, build, auto-close & clean)
  bun cli.ts orchestrate [serve|run|status]  (the Rust orchestrator: HTTP daemon, plan runner)
  bun cli.ts mobile [run|smoke|matrix|build|test|clean|syntax|hierarchy]
  bun cli.ts web [build|dev|lint|typecheck|translations]
  bun cli.ts backend [sync-types|test]
  bun cli.ts tui [verify|rust|wasm|web|backend|mobile]  (interactive TUI if no args)
  bun cli.ts translations [--dry-run|--locale=xx|--force]
  bun cli.ts verify
  bun cli.ts node-check
  bun cli.ts disk:heal [--force]
  bun cli.ts check
`);
      break;
  }
}

if (import.meta.main) {
  await main();
}
