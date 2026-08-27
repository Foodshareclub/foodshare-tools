/**
 * FoodShare Maestro E2E Test Suite Runner (Bun Powered)
 *
 * Replaces legacy bash scripts with a type-safe, resilient test orchestrator:
 * - Universal workspace directory resolution
 * - Automatically detects & targets booted simulator/emulator (--device <id>)
 * - Auto-heals disk storage before test runs
 * - Formats outputs, reports, and hierarchy dumps
 *
 * @module tools/maestro-runner
 */

import { existsSync, mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { autoHealDiskSpace } from "./clean-disk.ts";

function resolveAppDir(): string {
	if (existsSync(join(process.cwd(), ".maestro"))) {
		return process.cwd();
	}
	if (existsSync(join(process.cwd(), "..", "foodshare-app", ".maestro"))) {
		return join(process.cwd(), "..", "foodshare-app");
	}
	if (existsSync(join(process.cwd(), "foodshare-app", ".maestro"))) {
		return join(process.cwd(), "foodshare-app");
	}
	if (existsSync(join(import.meta.dir, "..", "foodshare-app", ".maestro"))) {
		return join(import.meta.dir, "..", "foodshare-app");
	}
	return join(import.meta.dir, "..");
}

const APP_DIR = resolveAppDir();
const MAESTRO_DIR = join(APP_DIR, ".maestro");
const REPORTS_DIR = join(MAESTRO_DIR, "reports");

async function getActiveDeviceId(): Promise<string | null> {
	// Check iOS simulators first
	try {
		const proc = Bun.spawn(["xcrun", "simctl", "list", "devices", "booted", "-j"], {
			stdout: "pipe",
			stderr: "ignore",
		});
		const output = await new Response(proc.stdout).text();
		const data = JSON.parse(output);
		for (const [runtime, devices] of Object.entries(data.devices as Record<string, unknown[]>)) {
			if (!runtime.includes("iOS")) continue;
			for (const device of devices as Array<{ udid: string; state: string }>) {
				if (device.state === "Booted") return device.udid;
			}
		}
	} catch {
		// ignore
	}

	// Check Android adb devices
	try {
		const adbProc = Bun.spawn(["adb", "devices"], {
			stdout: "pipe",
			stderr: "ignore",
		});
		const adbOut = await new Response(adbProc.stdout).text();
		const match = adbOut
			.split("\n")
			.slice(1)
			.find((l) => l.includes("\tdevice"));
		if (match) return match.split("\t")[0].trim();
	} catch {
		// ignore
	}

	return null;
}

async function runMaestroCommand(args: string[], deviceId?: string): Promise<number> {
	const activeId = deviceId || (await getActiveDeviceId());
	const cmd = ["maestro"];
	if (activeId) {
		cmd.push("--device", activeId);
	}
	cmd.push(...args);

	console.log(`🧪 Running: ${cmd.join(" ")}`);
	const proc = Bun.spawn(cmd, {
		stdout: "inherit",
		stderr: "inherit",
	});
	return await proc.exited;
}

async function checkSyntax(): Promise<number> {
	console.log("🔍 Validating Maestro test flow syntax...");
	const files: string[] = [];
	const EXCLUDED = new Set(["config.yaml", "config.yml", "maestro.yaml", "maestro.yml"]);

	if (existsSync(MAESTRO_DIR)) {
		for (const f of readdirSync(MAESTRO_DIR)) {
			if (EXCLUDED.has(f)) continue;
			if (f.endsWith(".yaml") || f.endsWith(".yml")) {
				files.push(join(MAESTRO_DIR, f));
			}
		}
	}

	const subflowsDir = join(MAESTRO_DIR, "subflows");
	if (existsSync(subflowsDir)) {
		for (const f of readdirSync(subflowsDir)) {
			if (EXCLUDED.has(f)) continue;
			if (f.endsWith(".yaml") || f.endsWith(".yml")) {
				files.push(join(subflowsDir, f));
			}
		}
	}

	let failed = 0;
	for (const file of files) {
		const checkProc = Bun.spawn(["maestro", "check-syntax", file], {
			stdout: "ignore",
			stderr: "ignore",
		});
		const code = await checkProc.exited;
		if (code !== 0) {
			console.error(`❌ Syntax error in ${file}`);
			failed++;
		} else {
			console.log(`✓ ${file.replace(APP_DIR, "")}`);
		}
	}

	if (failed > 0) {
		console.error(`❌ ${failed} flow(s) failed syntax validation.`);
		return 1;
	}
	console.log("✅ All Maestro flow files passed syntax validation!");
	return 0;
}

export async function main() {
	const args = process.argv.slice(2);
	const action = args[0] || "test";

	// Proactively clean disk space before running tests
	await autoHealDiskSpace(4.0);

	switch (action) {
		case "check": {
			const id = await getActiveDeviceId();
			console.log(`📱 Active target device: ${id || "None booted (run `bun sim:run` to launch)"}`);
			break;
		}

		case "syntax": {
			const code = await checkSyntax();
			process.exit(code);
			break;
		}

		case "smoke": {
			const code = await runMaestroCommand(["test", "--include-tags=smoke", MAESTRO_DIR]);
			process.exit(code);
			break;
		}

		case "regression": {
			const code = await runMaestroCommand(["test", "--include-tags=regression", MAESTRO_DIR]);
			process.exit(code);
			break;
		}

		case "critical": {
			const code = await runMaestroCommand(["test", "--include-tags=critical", MAESTRO_DIR]);
			process.exit(code);
			break;
		}

		case "a11y": {
			const code = await runMaestroCommand(["test", "--include-tags=a11y", MAESTRO_DIR]);
			process.exit(code);
			break;
		}

		case "test": {
			const code = await runMaestroCommand(["test", MAESTRO_DIR]);
			process.exit(code);
			break;
		}

		case "flow": {
			const flowFile = args[1];
			if (!flowFile) {
				console.error("❌ Please specify a flow file. Example: bun tools/maestro-runner.ts flow .maestro/01_onboarding_flow.yaml");
				process.exit(1);
			}
			const code = await runMaestroCommand(["test", flowFile]);
			process.exit(code);
			break;
		}

		case "report": {
			const format = (args[1] || "JUNIT").toUpperCase();
			mkdirSync(REPORTS_DIR, { recursive: true });
			const outFile = format === "JUNIT" ? join(REPORTS_DIR, "junit.xml") : join(REPORTS_DIR, `report.${format.toLowerCase()}`);
			const code = await runMaestroCommand(["test", `--format=${format}`, `--output=${outFile}`, MAESTRO_DIR]);
			console.log(`📊 Report generated at: ${outFile}`);
			process.exit(code);
			break;
		}

		case "hierarchy": {
			const code = await runMaestroCommand(["hierarchy"]);
			process.exit(code);
			break;
		}

		default: {
			console.log("Usage: bun tools/maestro-runner.ts [check|syntax|smoke|regression|critical|a11y|test|flow <file>|report <format>|hierarchy]");
			break;
		}
	}
}

if (import.meta.main) {
	await main();
}
