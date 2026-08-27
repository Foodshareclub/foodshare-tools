/**
 * FoodShare Universal Mobile Runner & Builder (Bun Powered)
 *
 * Programmatic, ultra-fast builder and simulator runner:
 * - Universal workspace directory resolution
 * - Proactive disk space auto-healing
 * - Programmatic iOS simulator discovery & booting
 * - Parallel xcodebuild compilation with Skip Swift engine
 * - Automatic app installation & launch
 * - Instant UI screenshot capture & visual verification
 * - Integrated Maestro E2E test runner with device targeting
 *
 * @module tools/sim-runner
 */

import { existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { autoHealDiskSpace } from "./clean-disk.ts";

function resolveAppDir(): string {
	if (existsSync(join(process.cwd(), "Darwin", "FoodShare.xcodeproj"))) {
		return process.cwd();
	}
	if (existsSync(join(process.cwd(), "..", "foodshare-app", "Darwin", "FoodShare.xcodeproj"))) {
		return join(process.cwd(), "..", "foodshare-app");
	}
	if (existsSync(join(process.cwd(), "foodshare-app", "Darwin", "FoodShare.xcodeproj"))) {
		return join(process.cwd(), "foodshare-app");
	}
	if (existsSync(join(import.meta.dir, "..", "foodshare-app", "Darwin", "FoodShare.xcodeproj"))) {
		return join(import.meta.dir, "..", "foodshare-app");
	}
	return join(import.meta.dir, "..");
}

const APP_DIR = resolveAppDir();
const DERIVED_DATA_PATH = join(APP_DIR, ".build", "DerivedData");
const BUNDLE_ID = "com.flutterflow.foodshare";

interface DeviceInfo {
	id: string;
	name: string;
	state: "Booted" | "Shutdown";
}

// =============================================================================
// Simulator Device Management
// =============================================================================

async function getAvailableSimulators(): Promise<{ booted: DeviceInfo[]; available: DeviceInfo[] }> {
	const proc = Bun.spawn(["xcrun", "simctl", "list", "devices", "-j"], {
		stdout: "pipe",
		stderr: "pipe",
	});
	const output = await new Response(proc.stdout).text();
	const data = JSON.parse(output);

	const booted: DeviceInfo[] = [];
	const available: DeviceInfo[] = [];

	for (const [runtime, devices] of Object.entries(data.devices as Record<string, unknown[]>)) {
		if (!runtime.includes("iOS")) continue;

		for (const device of devices as Array<{ udid: string; name: string; state: string; isAvailable: boolean }>) {
			if (!device.isAvailable) continue;
			const info: DeviceInfo = {
				id: device.udid,
				name: device.name,
				state: device.state === "Booted" ? "Booted" : "Shutdown",
			};

			if (device.state === "Booted") {
				booted.push(info);
			} else {
				available.push(info);
			}
		}
	}

	return { booted, available };
}

async function resolveOrBootSimulator(requestedId?: string): Promise<DeviceInfo> {
	const { booted, available } = await getAvailableSimulators();

	if (requestedId) {
		const target = [...booted, ...available].find((d) => d.id === requestedId);
		if (!target) throw new Error(`Requested simulator device ID ${requestedId} not found.`);
		if (target.state === "Shutdown") {
			console.log(`🚀 Booting requested simulator ${target.name} (${target.id})...`);
			const bootProc = Bun.spawn(["xcrun", "simctl", "boot", target.id]);
			await bootProc.exited;
			target.state = "Booted";
		}
		return target;
	}

	if (booted.length > 0) {
		console.log(`📱 Found active booted simulator: ${booted[0].name} (${booted[0].id})`);
		return booted[0];
	}

	// Find preferred iPhone model (17 Pro -> 16 Pro -> 15 Pro -> any iPhone)
	const preferred =
		available.find((d) => d.name.includes("iPhone 17 Pro")) ||
		available.find((d) => d.name.includes("iPhone 16 Pro")) ||
		available.find((d) => d.name.includes("iPhone 15 Pro")) ||
		available.find((d) => d.name.startsWith("iPhone")) ||
		available[0];

	if (!preferred) {
		throw new Error("No compatible iOS Simulator found on this system.");
	}

	console.log(`🚀 Booting simulator: ${preferred.name} (${preferred.id})...`);
	const bootProc = Bun.spawn(["xcrun", "simctl", "boot", preferred.id]);
	await bootProc.exited;
	preferred.state = "Booted";
	return preferred;
}

// =============================================================================
// Build & Deployment
// =============================================================================

async function buildIosApp(device: DeviceInfo, clean: boolean = false): Promise<string> {
	console.log("🔨 Compiling FoodShare App for iOS Simulator...");
	const startTime = performance.now();

	const args = [
		"xcodebuild",
		"-scheme",
		"FoodShare App",
		"-project",
		join(APP_DIR, "Darwin", "FoodShare.xcodeproj"),
		"-destination",
		`id=${device.id}`,
		"-configuration",
		"Debug",
		"-derivedDataPath",
		DERIVED_DATA_PATH,
		"SKIP_ACTION=none",
		"CODE_SIGN_IDENTITY=",
		"CODE_SIGNING_REQUIRED=NO",
		"CODE_SIGNING_ALLOWED=NO",
	];

	if (clean) {
		args.push("clean");
	}
	args.push("build");

	const buildProc = Bun.spawn(args, {
		stdout: "inherit",
		stderr: "inherit",
	});

	const exitCode = await buildProc.exited;
	if (exitCode !== 0) {
		throw new Error(`xcodebuild failed with exit code ${exitCode}`);
	}

	const appBundlePath = join(
		DERIVED_DATA_PATH,
		"Build",
		"Products",
		"Debug-iphonesimulator",
		"FoodShare.app",
	);

	if (!existsSync(appBundlePath)) {
		throw new Error(`App bundle not found at expected path: ${appBundlePath}`);
	}

	const tookSec = Math.round((performance.now() - startTime) / 100) / 10;
	console.log(`✅ Build Succeeded in ${tookSec}s! Bundle: ${appBundlePath}`);
	return appBundlePath;
}

async function installAndLaunch(device: DeviceInfo, appPath: string): Promise<number> {
	console.log(`📲 Installing FoodShare onto ${device.name}...`);
	const installProc = Bun.spawn(["xcrun", "simctl", "install", device.id, appPath]);
	const installCode = await installProc.exited;
	if (installCode !== 0) throw new Error(`Installation failed with code ${installCode}`);

	console.log(`🚀 Launching FoodShare (${BUNDLE_ID})...`);
	// Terminate any previous instance
	const termProc = Bun.spawn(["xcrun", "simctl", "terminate", device.id, BUNDLE_ID], {
		stdout: "ignore",
		stderr: "ignore",
	});
	await termProc.exited;

	const launchProc = Bun.spawn(["xcrun", "simctl", "launch", device.id, BUNDLE_ID], {
		stdout: "pipe",
	});
	const launchOut = await new Response(launchProc.stdout).text();
	console.log(`✅ App launched: ${launchOut.trim()}`);

	const pidMatch = launchOut.match(/: (\d+)/);
	return pidMatch ? Number.parseInt(pidMatch[1], 10) : 0;
}

async function captureScreenshot(device: DeviceInfo, outPath?: string): Promise<string> {
	const targetPath = outPath || join(APP_DIR, ".build", "simulator_screenshot.png");
	mkdirSync(join(APP_DIR, ".build"), { recursive: true });

	// Allow UI 1.5 seconds to settle
	await Bun.sleep(1500);

	const proc = Bun.spawn(["xcrun", "simctl", "io", device.id, "screenshot", targetPath], {
		stdout: "pipe",
		stderr: "pipe",
	});
	await proc.exited;
	console.log(`📸 UI Verification Screenshot captured: ${targetPath}`);
	return targetPath;
}

// =============================================================================
// CLI Entrypoint
// =============================================================================

export async function runSimFlow(options: {
	clean?: boolean;
	smoke?: boolean;
	test?: boolean;
	deviceId?: string;
	noBuild?: boolean;
}) {
	console.log("📱 =========================================================");
	console.log("📱 FoodShare Native iOS Programmatic Runner (Bun Powered)");
	console.log("📱 =========================================================");

	// 1. Proactive Disk Space Check & Auto-Healing
	await autoHealDiskSpace(4.0, options.clean);

	// 2. Discover / Boot Simulator
	const device = await resolveOrBootSimulator(options.deviceId);

	// 3. Build iOS Application
	let appPath = join(DERIVED_DATA_PATH, "Build", "Products", "Debug-iphonesimulator", "FoodShare.app");
	if (!options.noBuild || !existsSync(appPath)) {
		appPath = await buildIosApp(device, options.clean);
	}

	// 4. Install & Launch
	await installAndLaunch(device, appPath);

	// 5. Visual Verification Screenshot
	await captureScreenshot(device);

	// 6. Run Maestro E2E Suite if requested
	if (options.smoke || options.test) {
		console.log("\n🧪 Running Maestro E2E Suite against target simulator...");
		const maestroArgs = ["maestro", "--device", device.id, "test"];
		if (options.smoke) {
			maestroArgs.push("--include-tags=smoke");
		}
		maestroArgs.push(join(APP_DIR, ".maestro"));

		const maestroProc = Bun.spawn(maestroArgs, {
			stdout: "inherit",
			stderr: "inherit",
		});
		const maestroCode = await maestroProc.exited;
		if (maestroCode !== 0) {
			console.error(`❌ Maestro tests failed with exit code ${maestroCode}`);
			process.exit(maestroCode);
		}
		console.log("🎉 Maestro E2E tests passed successfully!");
	}

	console.log("🎉 Programmatic execution complete!");
}

if (import.meta.main) {
	const args = process.argv.slice(2);
	const clean = args.includes("--clean");
	const smoke = args.includes("--smoke");
	const test = args.includes("--test");
	const noBuild = args.includes("--no-build");
	const deviceArg = args.find((a) => a.startsWith("--device="))?.split("=")[1];

	runSimFlow({ clean, smoke, test, noBuild, deviceId: deviceArg }).catch((err) => {
		console.error(`❌ Fatal Runner Error: ${err instanceof Error ? err.message : String(err)}`);
		process.exit(1);
	});
}
