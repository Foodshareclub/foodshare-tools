/**
 * Disk Space Health Guard & Auto-Healer for FoodShare
 *
 * Automatically monitors and reclaims disk space before builds and test runs:
 * - Cleans old Maestro test runs and screen recordings (~/.maestro/tests)
 * - Prunes old Xcode DerivedData logs and intermediate artifacts
 * - Clears temp caches
 */

import { existsSync, readdirSync, rmSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export interface DiskStats {
	totalGb: number;
	usedGb: number;
	availGb: number;
	availPercent: number;
}

export async function getDiskStats(targetPath: string = process.cwd()): Promise<DiskStats> {
	const proc = Bun.spawn(["df", "-k", targetPath], {
		stdout: "pipe",
		stderr: "pipe",
	});
	const output = await new Response(proc.stdout).text();
	const lines = output.trim().split("\n");
	if (lines.length < 2) {
		return { totalGb: 0, usedGb: 0, availGb: 0, availPercent: 0 };
	}

	const parts = lines[1].replace(/\s+/g, " ").split(" ");
	// 1K-blocks: total = parts[1], used = parts[2], avail = parts[3]
	const totalKb = Number.parseInt(parts[1], 10) || 0;
	const usedKb = Number.parseInt(parts[2], 10) || 0;
	const availKb = Number.parseInt(parts[3], 10) || 0;

	return {
		totalGb: Math.round((totalKb / 1024 / 1024) * 10) / 10,
		usedGb: Math.round((usedKb / 1024 / 1024) * 10) / 10,
		availGb: Math.round((availKb / 1024 / 1024) * 10) / 10,
		availPercent: totalKb > 0 ? Math.round((availKb / totalKb) * 100) : 0,
	};
}

export async function autoHealDiskSpace(minFreeGb: number = 4.0, force: boolean = false): Promise<boolean> {
	const beforeStats = await getDiskStats();
	console.log(`💾 Current Free Disk Space: ${beforeStats.availGb} GB (${beforeStats.availPercent}% free)`);

	if (!force && beforeStats.availGb >= minFreeGb) {
		return false;
	}

	console.log(`🧹 Auto-healing disk space (threshold: ${minFreeGb} GB)...`);
	let reclaimedCount = 0;

	// 1. Clean Maestro test logs and recordings
	const maestroTestsDir = join(homedir(), ".maestro", "tests");
	if (existsSync(maestroTestsDir)) {
		try {
			const entries = readdirSync(maestroTestsDir);
			for (const entry of entries) {
				const fullPath = join(maestroTestsDir, entry);
				rmSync(fullPath, { recursive: true, force: true });
				reclaimedCount++;
			}
			console.log(`  ✓ Cleaned ${entries.length} Maestro test recordings from ~/.maestro/tests`);
		} catch (err) {
			console.warn(`  ⚠ Could not clean Maestro tests: ${err instanceof Error ? err.message : String(err)}`);
		}
	}

	// 2. Clean temporary macOS simulator and XCTest caches
	const xctestCache = join(homedir(), "Library", "Caches", "com.apple.dt.XCTest");
	if (existsSync(xctestCache)) {
		try {
			rmSync(xctestCache, { recursive: true, force: true });
			console.log("  ✓ Cleaned XCTest test cache");
		} catch {
			// ignore
		}
	}

	// 3. Clean Bun package cache if force or critical
	const bunCacheDir = join(homedir(), ".bun", "install", "cache");
	if (existsSync(bunCacheDir) && (force || beforeStats.availGb < 2.0)) {
		try {
			rmSync(bunCacheDir, { recursive: true, force: true });
			console.log("  ✓ Pruned Bun install cache");
		} catch {
			// ignore
		}
	}

	// 4. Clean FoodShare old DerivedData logs if too large
	const derivedDataLogs = join(process.cwd(), ".build", "DerivedData", "Logs");
	if (existsSync(derivedDataLogs)) {
		try {
			rmSync(derivedDataLogs, { recursive: true, force: true });
			console.log("  ✓ Pruned old Xcode compilation build logs");
		} catch {
			// ignore
		}
	}

	// 5. Clean Cargo incremental debug caches if disk space is tight
	const cargoIncrementalDir = join(process.cwd(), "target", "debug", "incremental");
	const cargoIncrementalToolsDir = join(process.cwd(), "foodshare-tools", "target", "debug", "incremental");
	for (const incDir of [cargoIncrementalDir, cargoIncrementalToolsDir]) {
		if (existsSync(incDir) && (force || beforeStats.availGb < minFreeGb)) {
			try {
				rmSync(incDir, { recursive: true, force: true });
				console.log("  ✓ Pruned Cargo incremental debug cache");
			} catch {
				// ignore
			}
		}
	}

	const afterStats = await getDiskStats();
	const freedGb = Math.max(0, Math.round((afterStats.availGb - beforeStats.availGb) * 10) / 10);
	console.log(`✨ Disk cleanup complete: Reclaimed ~${freedGb} GB. Available: ${afterStats.availGb} GB.`);
	return true;
}

if (import.meta.main) {
	const force = process.argv.includes("--force");
	await autoHealDiskSpace(4.0, force);
}
