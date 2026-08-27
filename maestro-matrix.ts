/**
 * Multi-Device Parallel Matrix Runner for Maestro E2E Suites (Bun Powered)
 *
 * Executes test flows concurrently across all active iOS Simulators and Android Emulators.
 * - Discovers all booted iOS devices via `xcrun simctl`
 * - Discovers all connected Android devices via `adb devices`
 * - Runs Maestro in parallel using Bun.spawn with isolated log files
 * - Summarizes execution time and results in a structured terminal matrix
 *
 * @module tools/maestro-matrix
 */

import { existsSync, mkdirSync } from "node:fs";
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

interface TargetDevice {
	platform: "ios" | "android";
	id: string;
	name: string;
}

// =============================================================================
// Device Discovery
// =============================================================================

async function discoverDevices(): Promise<TargetDevice[]> {
	const devices: TargetDevice[] = [];

	// 1. Discover iOS Simulators
	try {
		const proc = Bun.spawn(["xcrun", "simctl", "list", "devices", "booted", "-j"], {
			stdout: "pipe",
			stderr: "ignore",
		});
		const output = await new Response(proc.stdout).text();
		const data = JSON.parse(output);

		for (const [runtime, devList] of Object.entries(data.devices as Record<string, unknown[]>)) {
			if (!runtime.includes("iOS")) continue;
			for (const dev of devList as Array<{ udid: string; name: string; state: string }>) {
				if (dev.state === "Booted") {
					devices.push({
						platform: "ios",
						id: dev.udid,
						name: dev.name,
					});
				}
			}
		}
	} catch {
		// ignore
	}

	// 2. Discover Android Emulators
	try {
		const adbProc = Bun.spawn(["adb", "devices", "-l"], {
			stdout: "pipe",
			stderr: "ignore",
		});
		const adbOut = await new Response(adbProc.stdout).text();
		const lines = adbOut.split("\n").slice(1);
		for (const line of lines) {
			if (line.includes("device") && !line.includes("offline")) {
				const parts = line.trim().split(/\s+/);
				const serial = parts[0];
				if (serial && serial !== "List") {
					const modelMatch = line.match(/model:(\S+)/);
					devices.push({
						platform: "android",
						id: serial,
						name: modelMatch ? modelMatch[1] : serial,
					});
				}
			}
		}
	} catch {
		// ignore
	}

	return devices;
}

// =============================================================================
// Matrix Execution Engine
// =============================================================================

interface MatrixResult {
	device: TargetDevice;
	passed: boolean;
	durationSec: number;
	logFile: string;
	error?: string;
}

async function runDeviceFlow(
	device: TargetDevice,
	extraArgs: string[],
	targetPath: string,
): Promise<MatrixResult> {
	mkdirSync(REPORTS_DIR, { recursive: true });
	const safeId = device.id.replace(/[:/\\ ]/g, "_");
	const logFile = join(REPORTS_DIR, `matrix_${device.platform}_${safeId}.log`);
	const startTime = performance.now();

	console.log(`🚀 [${device.platform.toUpperCase()}] Starting Maestro on ${device.name} (${device.id})...`);

	const cmd = ["maestro", "--device", device.id, "test", ...extraArgs, targetPath];

	const logFd = Bun.file(logFile).writer();

	const proc = Bun.spawn(cmd, {
		stdout: "pipe",
		stderr: "pipe",
	});

	// Stream stdout & stderr to log file
	const streamPromise = (async () => {
		const reader = proc.stdout.getReader();
		while (true) {
			const { done, value } = await reader.read();
			if (done) break;
			logFd.write(value);
		}
	})();

	const errStreamPromise = (async () => {
		const reader = proc.stderr.getReader();
		while (true) {
			const { done, value } = await reader.read();
			if (done) break;
			logFd.write(value);
		}
	})();

	const exitCode = await proc.exited;
	await Promise.all([streamPromise, errStreamPromise]);
	logFd.end();

	const durationSec = Math.round((performance.now() - startTime) / 100) / 10;
	const passed = exitCode === 0;

	if (passed) {
		console.log(`✅ [${device.platform.toUpperCase()}] ${device.name} PASSED in ${durationSec}s`);
	} else {
		console.error(`❌ [${device.platform.toUpperCase()}] ${device.name} FAILED in ${durationSec}s (see ${logFile})`);
	}

	return {
		device,
		passed,
		durationSec,
		logFile,
		error: passed ? undefined : `Exit code ${exitCode}`,
	};
}

export async function runMatrix(args: string[] = []) {
	console.log("📱 =========================================================");
	console.log("📱 FoodShare Multi-Device Parallel Matrix Runner (Bun)");
	console.log("📱 =========================================================");

	// 1. Clean disk space before running multi-device suites
	await autoHealDiskSpace(4.0);

	// 2. Discover active devices
	const devices = await discoverDevices();

	let extraArgs: string[] = [];
	let targetPath = MAESTRO_DIR;

	if (args.length > 0) {
		const tag = args[0];
		if (["smoke", "regression", "critical", "a11y"].includes(tag)) {
			extraArgs.push(`--include-tags=${tag}`);
		} else if (existsSync(tag)) {
			targetPath = tag;
		} else {
			extraArgs = args;
		}
	}

	if (devices.length === 0) {
		console.warn("⚠️ No booted iOS Simulators or connected Android Emulators found.");
		console.log("Defaulting to single-target Maestro execution...");
		const proc = Bun.spawn(["maestro", "test", ...extraArgs, targetPath], {
			stdout: "inherit",
			stderr: "inherit",
		});
		const code = await proc.exited;
		process.exit(code);
	}

	console.log(`Found ${devices.length} active target device(s):`);
	for (const dev of devices) {
		console.log(`  • [${dev.platform.toUpperCase()}] ${dev.name} (${dev.id})`);
	}

	// 3. Concurrently execute Maestro test suites across all devices
	const startTime = performance.now();
	const results = await Promise.all(
		devices.map((device) => runDeviceFlow(device, extraArgs, targetPath)),
	);
	const totalSec = Math.round((performance.now() - startTime) / 100) / 10;

	// 4. Aggregate Matrix Summary
	const passedList = results.filter((r) => r.passed);
	const failedList = results.filter((r) => !r.passed);

	console.log("\n==========================================");
	console.log("        MAESTRO MATRIX TEST SUMMARY       ");
	console.log("==========================================");
	console.log(` Total Devices:  ${devices.length}`);
	console.log(` Passed:         ${passedList.length}`);
	console.log(` Failed:         ${failedList.length}`);
	console.log(` Total Duration: ${totalSec}s`);
	console.log("==========================================");

	for (const res of results) {
		const icon = res.passed ? "✅" : "❌";
		console.log(` ${icon} [${res.device.platform.toUpperCase()}] ${res.device.name}: ${res.durationSec}s`);
	}
	console.log("==========================================\n");

	if (failedList.length > 0) {
		process.exit(1);
	}
	process.exit(0);
}

if (import.meta.main) {
	await runMatrix(process.argv.slice(2));
}
