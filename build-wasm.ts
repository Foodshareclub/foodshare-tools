/**
 * FoodShare WebAssembly (WASM) Build Engine (Bun Powered)
 *
 * Compiles Rust crates into optimized WebAssembly modules with TypeScript bindings:
 * - `foodshare-geo` -> WASM (Haversine distance, PostGIS point parser)
 * - `foodshare-search` -> WASM (Vector cosine similarity, L2 distance, RRF, fuzzy search)
 * - `foodshare-crypto` -> WASM (TOTP MFA generation/verification, HMAC-SHA256)
 * - `foodshare-compression` -> WASM (Brotli/Gzip, ETag) + native node:zlib fallback
 * - `foodshare-image` -> WASM (format detection, geometry)
 *
 * Target `nodejs` is intentional: bridges in `foodshare-web/src/lib/wasm-*.ts`
 * are server-only (`import "server-only"`). Do not switch to `bundler`/`web`
 * without migrating bridges to async `await init()` + dynamic import.
 * Size is optimized via `wasm-opt = ["-Oz"]` in each crate Cargo.toml.
 *
 * @module tools/build-wasm
 */

import {
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";

function postProcessWasmJs(outDir: string, crateName: string) {
  const jsName = `${crateName.replace(/-/g, "_")}.js`;
  const wasmBgName = `${crateName.replace(/-/g, "_")}_bg.wasm`;
  const jsPath = join(outDir, jsName);
  if (!existsSync(jsPath)) return;

  let content = readFileSync(jsPath, "utf-8");
  const oldLoader = `const wasmPath = \`\${__dirname}/${wasmBgName}\`;\nconst wasmBytes = require('fs').readFileSync(wasmPath);`;

  const universalLoader = `
function __resolveWasmPath(filename) {
    const fs = require('fs');
    const path = require('path');
    const candidates = [
        path.join(__dirname, filename),
        path.join(process.cwd(), "src", "wasm", "${crateName}", filename),
        path.join(process.cwd(), "foodshare-web", "src", "wasm", "${crateName}", filename),
        path.join(process.cwd(), "..", "foodshare-web", "src", "wasm", "${crateName}", filename),
    ];
    for (const c of candidates) {
        if (fs.existsSync(/*turbopackIgnore: true*/ c)) return c;
    }
    return candidates[0];
}
const wasmPath = __resolveWasmPath('${wasmBgName}');
const wasmBytes = require('fs').readFileSync(/*turbopackIgnore: true*/ wasmPath);`;

  if (content.includes(oldLoader)) {
    content = content.replace(oldLoader, universalLoader.trim());
    writeFileSync(jsPath, content, "utf-8");
    console.log(
      `  ✓ Injected universal Turbopack/Next.js path resolver for ${jsName}`,
    );
  }
}

function resolveWorkspaceRoot(): string {
  if (existsSync(join(process.cwd(), "crates"))) {
    return process.cwd();
  }
  if (existsSync(join(process.cwd(), "foodshare-tools", "crates"))) {
    return join(process.cwd(), "foodshare-tools");
  }
  return process.cwd();
}

const TOOLS_DIR = resolveWorkspaceRoot();
const WASM_OUT_DIR = join(TOOLS_DIR, "..", "foodshare-web", "src", "wasm");

const CRATES = [
  { name: "foodshare-geo", path: "crates/geo" },
  { name: "foodshare-search", path: "crates/search" },
  { name: "foodshare-crypto", path: "crates/crypto" },
  { name: "foodshare-compression", path: "crates/compression" },
  { name: "foodshare-image", path: "crates/image" },
];

async function compileWasmCrate(
  crate: { name: string; path: string },
  target: "web" | "nodejs" | "bundler" = "bundler",
) {
  console.log(`🦀 Building WASM for [${crate.name}] (${target})...`);
  const crateDir = join(TOOLS_DIR, crate.path);
  const outDir = join(WASM_OUT_DIR, crate.name);

  mkdirSync(outDir, { recursive: true });

  // wasm-pack >= 0.13 required for wasm-opt -Oz + bulk-memory support.
  try {
    const v = Bun.spawnSync(["wasm-pack", "--version"], { stdout: "pipe" });
    const version = new TextDecoder().decode(v.stdout).trim();
    console.log(`  wasm-pack: ${version || "unknown"}`);
  } catch {
    console.warn("  ⚠️ could not detect wasm-pack version (need >= 0.13)");
  }

  const cmd = [
    "wasm-pack",
    "build",
    crateDir,
    "--target",
    target,
    "--out-dir",
    outDir,
    "--release",
    "--",
    "--locked",
    "--features",
    "wasm",
  ];

  const startTime = performance.now();
  const proc = Bun.spawn(cmd, {
    cwd: TOOLS_DIR,
    stdout: "inherit",
    stderr: "inherit",
  });

  const code = await proc.exited;
  const durationSec = Math.round((performance.now() - startTime) / 100) / 10;

  if (code === 0) {
    postProcessWasmJs(outDir, crate.name);
    try {
      const wasmFile = join(outDir, `${crate.name.replace(/-/g, "_")}_bg.wasm`);
      const kb = (statSync(wasmFile).size / 1024).toFixed(1);
      console.log(
        `✅ [${crate.name}] WASM compiled successfully in ${durationSec}s -> ${outDir} (${kb} KB, wasm-opt -Oz)`,
      );
    } catch {
      console.log(
        `✅ [${crate.name}] WASM compiled successfully in ${durationSec}s -> ${outDir}`,
      );
    }
    return true;
  } else {
    console.error(`❌ [${crate.name}] WASM compilation failed (code ${code})`);
    return false;
  }
}

export async function main() {
  console.log("🦀 =========================================================");
  console.log("🦀 FoodShare Rust -> WebAssembly (WASM) Build Engine (Bun)");
  console.log("🦀 =========================================================");

  // Parallel builds reuse shared cargo target cache; use --quick path in cli.ts to skip when cached.
  const results = await Promise.all(
    CRATES.map((crate) => compileWasmCrate(crate, "nodejs")),
  );
  const allSuccess = results.every(Boolean);

  console.log("\n==========================================");
  if (allSuccess) {
    console.log("🎉 ALL RUST WASM MODULES COMPILED 100% GREEN!");
  } else {
    console.error("❌ Some WASM modules failed to compile.");
    process.exit(1);
  }
  console.log("==========================================\n");
}

if (import.meta.main) {
  await main();
}
