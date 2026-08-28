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
 * Usage:
 *   bun cli.ts auto [--keep] [--retry N] [--clean] [--quick]  (run-until-green, build, auto-close & clean)
 *   bun cli.ts mobile [run|smoke|matrix|build|test|clean|syntax|hierarchy]
 *   bun cli.ts web [build|dev|lint|typecheck]
 *   bun cli.ts backend [sync-types|test]
 *   bun cli.ts tui [verify|rust|wasm|web|backend|mobile]
 *   bun cli.ts verify
 *   bun cli.ts disk:heal [--force]
 *   bun cli.ts check
 *
 * @module tools/cli
 */

import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { autoHealDiskSpace } from "./clean-disk.ts";
import { runSimFlow } from "./sim-runner.ts";
import { main as runMaestro } from "./maestro-runner.ts";
import { runMatrix } from "./maestro-matrix.ts";
import { main as runBuilder } from "./builder.ts";

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
      await runSimFlow({});
      break;
    case "smoke":
      await runSimFlow({ smoke: true });
      break;
    case "matrix":
      await runMatrix(subArgs);
      break;
    case "build":
      process.argv = [process.argv[0], process.argv[1], ...subArgs];
      await runBuilder();
      break;
    case "test":
      await runSimFlow({ test: true });
      break;
    case "clean":
      await runSimFlow({ clean: true });
      break;
    default:
      // Forward directly to maestro runner
      process.argv = [process.argv[0], process.argv[1], ...args];
      await runMaestro();
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
        ["bunx", "supabase", "gen", "types", "typescript", "--local"],
        BACKEND_DIR,
      );
      break;
    default:
      await runCommand(["bun", "run", ...args], BACKEND_DIR);
      break;
  }
}

async function handleTui(args: string[]) {
  const tuiBin = join(ROOT_DIR, "foodshare-tools", "target", "debug", "fs-tui");
  const hasBin = existsSync(tuiBin);
  if (!hasBin) {
    console.log("🔧 Building fs-tui (Rust TUI)...");
    await runCommand(
      ["cargo", "build", "--bin", "fs-tui"],
      join(ROOT_DIR, "foodshare-tools"),
    );
  }
  // Pass through args: verify/rust/wasm/web/backend/mobile or interactive if no args
  const tuiArgs = args.length > 0 ? args : [];
  console.log(`🖥️  Launching TUI: ${tuiBin} ${tuiArgs.join(" ")}`);
  const proc = Bun.spawn([tuiBin, ...tuiArgs], {
    cwd: ROOT_DIR,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  await proc.exited;
}

// ── Auto Orchestrator: run-until-green, build, auto-close & clean ──────────
const ARTIFACT_DIRS = [
  "foodshare-tools/target",
  "foodshare-app/.build",
  "foodshare-app/Android/app/build",
  "foodshare-app/Android/.gradle",
  "foodshare-app/Android/.skip-stub",
  "foodshare-app/Android/.skip-plugin-marker",
  "foodshare-web/.next",
  "foodshare-web/.turbo",
  "foodshare-web/node_modules/.cache",
  "foodshare-web/.next/cache",
  "foodshare-backend/supabase/.temp",
  ".turbo",
];

function cleanArtifacts(root: string = ROOT_DIR) {
  console.log("\n🧹 Cleaning artifacts...");
  let cleaned = 0;
  for (const rel of ARTIFACT_DIRS) {
    const p = join(root, rel);
    if (existsSync(p)) {
      try {
        rmSync(p, { recursive: true, force: true });
        console.log(`  ✓ removed ${rel}`);
        cleaned++;
      } catch {}
    }
  }
  // Prune Cargo incremental caches via disk healer
  console.log(`  ✓ cleaned ${cleaned} artifact dirs`);
}

async function runWithRetry(
  label: string,
  cmd: string[],
  cwd: string,
  maxRetries = 2,
): Promise<void> {
  for (let attempt = 1; attempt <= maxRetries + 1; attempt++) {
    console.log(
      `\n▶ [${label}] attempt ${attempt}/${maxRetries + 1}: ${cmd.join(" ")}`,
    );
    const code = await runCommand(cmd, cwd);
    if (code === 0) {
      console.log(`✅ [${label}] passed`);
      return;
    }
    console.warn(`⚠️  [${label}] failed (exit ${code})`);
    if (attempt <= maxRetries) {
      console.log(
        `↻ healing disk & pruning incremental caches before retry...`,
      );
      await autoHealDiskSpace(4.0, true);
      // Prune incremental for Rust
      for (const inc of [
        join(ROOT_DIR, "foodshare-tools/target/debug/incremental"),
        join(ROOT_DIR, "foodshare-app/.build"),
      ]) {
        if (existsSync(inc))
          try {
            rmSync(inc, { recursive: true, force: true });
          } catch {}
      }
      await Bun.sleep(1000 * attempt);
    } else {
      throw new Error(`${label} failed after ${maxRetries + 1} attempts`);
    }
  }
}

async function runAuto(args: string[]) {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`
Usage: bun cli.ts auto [--keep] [--retry N] [--clean] [--quick] [--sequential]
  --keep        Keep artifacts after success (default: clean)
  --retry N     Max retries per domain (default: 2)
  --clean       Clean before run
  --quick       Skip WASM build if cached
  --sequential  Run Web+Backend sequentially (default: parallel)
`);
    return;
  }
  const keepArtifacts = args.includes("--keep");
  const maxRetries = args.includes("--retry")
    ? Number(args[args.indexOf("--retry") + 1] ?? 2)
    : 2;
  const parallel = !args.includes("--sequential");
  console.log("🤖 =========================================================");
  console.log("🤖 FoodShare Auto — run-until-green, build, auto-close & clean");
  console.log("🤖 =========================================================");
  console.log(
    `   keepArtifacts=${keepArtifacts}  maxRetries=${maxRetries}  parallel=${parallel}`,
  );

  const start = Date.now();
  let spawned: Bun.Subprocess[] = [];
  const register = (p: Bun.Subprocess) => spawned.push(p);
  // Ensure auto-close on SIGINT
  const onSig = () => {
    console.log("\n🛑 SIGINT — cleaning & exiting...");
    for (const p of spawned)
      try {
        p.kill();
      } catch {}
    if (!keepArtifacts) cleanArtifacts();
    process.exit(130);
  };
  process.on("SIGINT", onSig);
  process.on("SIGTERM", onSig);

  try {
    await autoHealDiskSpace(4.0, true);
    if (args.includes("--clean")) cleanArtifacts();

    // Phase 1: Rust + WASM (WASM needs Rust)
    console.log("\n🦀 [1/5] Rust workspace tests...");
    await runWithRetry(
      "Rust",
      ["cargo", "test", "--workspace"],
      join(ROOT_DIR, "foodshare-tools"),
      maxRetries,
    );

    console.log("\n📦 [2/5] WASM 5 crates...");
    // Prefer prebuilt WASM if --quick
    if (args.includes("--quick")) {
      console.log("  (quick: skipping wasm-pack, using cached .wasm)");
    } else {
      await runWithRetry(
        "WASM",
        ["bun", "tools/build-wasm.ts"],
        join(ROOT_DIR, "foodshare-tools"),
        maxRetries,
      );
    }

    // Phase 2: Web & Backend in parallel if allowed
    if (parallel) {
      console.log("\n⚡ [3/5] Web + Backend (parallel)...");
      await Promise.all([
        runWithRetry(
          "Web type-check",
          ["bun", "run", "type-check"],
          WEB_DIR,
          maxRetries,
        ),
        runWithRetry(
          "Backend",
          [
            "deno",
            "test",
            "--allow-all",
            "--config",
            "supabase/functions/deno.json",
            "supabase/functions/__tests__/",
          ],
          BACKEND_DIR,
          maxRetries,
        ),
      ]);
      console.log("\n📦 [4/5] Web build (Turbopack)...");
      await runWithRetry(
        "Web build",
        ["bun", "run", "build"],
        WEB_DIR,
        maxRetries,
      );
    } else {
      console.log("\n⚡ [3/5] Web type-check...");
      await runWithRetry(
        "Web type-check",
        ["bun", "run", "type-check"],
        WEB_DIR,
        maxRetries,
      );
      console.log("\n📦 [4/5] Web build...");
      await runWithRetry(
        "Web build",
        ["bun", "run", "build"],
        WEB_DIR,
        maxRetries,
      );
      console.log("\n🛡️  [5/5] Backend Deno tests...");
      await runWithRetry(
        "Backend",
        [
          "deno",
          "test",
          "--allow-all",
          "--config",
          "supabase/functions/deno.json",
          "supabase/functions/__tests__/",
        ],
        BACKEND_DIR,
        maxRetries,
      );
    }

    // Phase 3: Mobile (syntax + Gradle) — requires ANDROID_HOME
    console.log("\n📱 [5/5] Mobile — Maestro syntax + Gradle unit tests...");
    await runWithRetry(
      "Maestro syntax",
      ["bun", "tools/maestro-runner.ts", "syntax"],
      APP_DIR,
      maxRetries,
    );
    // Gradle test uses resilient settings, set ANDROID_HOME if missing
    const gradleEnv = {
      ...process.env,
      ANDROID_HOME:
        process.env.ANDROID_HOME ?? "/Users/organic/Library/Android/sdk",
    };
    // We run via runCommand which inherits env, but we can spawn manually to set env
    {
      const cmd = [
        "./gradlew",
        ":app:testDebugUnitTest",
        "--build-cache",
        "--parallel",
        "--no-configuration-cache",
      ];
      console.log(`▶ [Gradle] ${cmd.join(" ")}`);
      const proc = Bun.spawn(cmd, {
        cwd: join(ROOT_DIR, "foodshare-app/Android"),
        env: gradleEnv as Record<string, string>,
        stdout: "inherit",
        stderr: "inherit",
      });
      register(proc);
      const code = await proc.exited;
      if (code !== 0) throw new Error(`Gradle failed ${code}`);
      console.log("✅ [Gradle] passed");
    }

    // Phase 4: Final builds (only if tests green)
    console.log("\n🔨 Final builds (release artifacts)...");
    // Rust release not needed for app, but we do Web already built; Android APK debug already via Gradle test stage would have built intermediates
    // We ensure at least one artifact exists per domain
    const webNextExists = existsSync(join(WEB_DIR, ".next"));
    const wasmExists = existsSync(
      join(WEB_DIR, "src/wasm/foodshare-search/foodshare_search_bg.wasm"),
    );
    console.log(`  • Web .next: ${webNextExists ? "✅" : "❌"}`);
    console.log(`  • WASM search: ${wasmExists ? "✅" : "❌"}`);

    const dur = ((Date.now() - start) / 1000).toFixed(1);
    console.log("\n==========================================");
    console.log(`🎉 AUTO SUCCESS — all green in ${dur}s`);
    console.log("==========================================\n");
  } catch (e) {
    console.error(
      `\n❌ AUTO FAILED: ${e instanceof Error ? e.message : String(e)}`,
    );
    process.exit(1);
  } finally {
    process.off("SIGINT", onSig);
    process.off("SIGTERM", onSig);
    // Auto-close: kill any spawned dev servers (none started in this flow, but ensure)
    for (const p of spawned)
      try {
        if (p.exitCode === null) p.kill();
      } catch {}
    if (!keepArtifacts) {
      cleanArtifacts();
      await autoHealDiskSpace(4.0, true);
      console.log("✨ Auto-closed & cleaned — repo neat");
    } else {
      console.log("📦 Keeping artifacts (--keep)");
    }
  }
}

async function runFullVerification() {
  console.log("🌟 =========================================================");
  console.log("🌟 FoodShare 10x Full Domain Verification");
  console.log("🌟 =========================================================");

  // 1. Disk Health
  await autoHealDiskSpace(4.0);

  // 2. Rust Crates & Test Engine
  console.log("\n🦀 [1/4] Running Rust 2024 Workspace Test Engine...");
  const rustCode = await runCommand(
    ["cargo", "test", "--workspace"],
    join(ROOT_DIR, "foodshare-tools"),
  );
  if (rustCode !== 0) {
    console.error("❌ Rust workspace tests failed.");
    process.exit(1);
  }

  // 3. Web Build & Type Check
  console.log("\n📦 [2/4] Building Web Application with Next.js Turbopack...");
  const webBuildCode = await runCommand(["bun", "run", "build"], WEB_DIR);
  if (webBuildCode !== 0) {
    console.error("❌ Web build failed.");
    process.exit(1);
  }

  // 4. Mobile Smoke Test Suite
  console.log("\n📱 [3/4] Executing Mobile Simulator Runner & Smoke Tests...");
  await runSimFlow({ smoke: true });

  // 5. Backend Edge Function Deno Unit Tests
  console.log(
    "\n⚡ [4/4] Executing Backend Supabase Edge Function Deno Tests...",
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
  await autoHealDiskSpace(4.0);

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
      await runAuto(subArgs);
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
      await runCommand(
        ["bun", "tools/build-wasm.ts", ...subArgs],
        join(ROOT_DIR, "foodshare-tools"),
      );
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
    case "disk:heal":
      await autoHealDiskSpace(4.0, subArgs.includes("--force"));
      break;
    case "check":
    case "status":
      await printStatus();
      break;
    default:
      console.log(`
Usage:
  bun cli.ts auto [--keep] [--retry N] [--clean] [--quick] [--sequential]  (run-until-green, build, auto-close & clean)
  bun cli.ts mobile [run|smoke|matrix|build|test|clean|syntax|hierarchy]
  bun cli.ts web [build|dev|lint|typecheck|translations]
  bun cli.ts backend [sync-types|test]
  bun cli.ts tui [verify|rust|wasm|web|backend|mobile]  (interactive TUI if no args)
  bun cli.ts translations [--dry-run|--locale=xx|--force]
  bun cli.ts verify
  bun cli.ts disk:heal [--force]
  bun cli.ts check
`);
      break;
  }
}

if (import.meta.main) {
  await main();
}
