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
 *   bun cli.ts mobile [run|smoke|matrix|build|test|clean|syntax|hierarchy]
 *   bun cli.ts web [build|dev|lint|typecheck]
 *   bun cli.ts backend [sync-types|test]
 *   bun cli.ts verify
 *   bun cli.ts disk:heal [--force]
 *   bun cli.ts check
 *
 * @module tools/cli
 */

import { existsSync } from "node:fs";
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

async function runCommand(cmd: string[], cwd: string = ROOT_DIR): Promise<number> {
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
			await runCommand(["bun", "run", "translations:sync", ...subArgs], WEB_DIR);
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
				["deno", "test", "--allow-all", "--config", "supabase/functions/deno.json", "supabase/functions/__tests__/"],
				BACKEND_DIR,
			);
			break;
		case "sync-types":
			await runCommand(["bunx", "supabase", "gen", "types", "typescript", "--local"], BACKEND_DIR);
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
		await runCommand(["cargo", "build", "--bin", "fs-tui"], join(ROOT_DIR, "foodshare-tools"));
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

async function runFullVerification() {
	console.log("🌟 =========================================================");
	console.log("🌟 FoodShare 10x Full Domain Verification");
	console.log("🌟 =========================================================");

	// 1. Disk Health
	await autoHealDiskSpace(4.0);

	// 2. Rust Crates & Test Engine
	console.log("\n🦀 [1/4] Running Rust 2024 Workspace Test Engine...");
	const rustCode = await runCommand(["cargo", "test", "--workspace"], join(ROOT_DIR, "foodshare-tools"));
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
	console.log("\n⚡ [4/4] Executing Backend Supabase Edge Function Deno Tests...");
	const backendCode = await runCommand(
		["deno", "test", "--allow-all", "--config", "supabase/functions/deno.json", "supabase/functions/__tests__/"],
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
	console.log(`  • FoodShare App:     ${existsSync(APP_DIR) ? "✅ Ready" : "❌ Missing"}`);
	console.log(`  • FoodShare Web:     ${existsSync(WEB_DIR) ? "✅ Ready" : "❌ Missing"}`);
	console.log(`  • FoodShare Backend: ${existsSync(BACKEND_DIR) ? "✅ Ready" : "❌ Missing"}`);
}

async function main() {
	const args = process.argv.slice(2);
	const domain = args[0] || "check";
	const subArgs = args.slice(1);

	switch (domain) {
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
			await runCommand(["cargo", "test", "--workspace", ...subArgs], join(ROOT_DIR, "foodshare-tools"));
			break;
		case "wasm":
			await runCommand(["bun", "tools/build-wasm.ts", ...subArgs], join(ROOT_DIR, "foodshare-tools"));
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
			await runCommand(["bun", "run", "translations:sync", ...subArgs], WEB_DIR);
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
