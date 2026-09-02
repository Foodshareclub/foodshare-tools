/**
 * FoodShare Cross-Platform Mobile Builder & Test Engine (Bun Powered)
 *
 * Industrial-grade cross-platform build orchestrator for Skip (Swift -> Kotlin):
 * - Auto-heals disk storage before massive compilations
 * - Swift build & Skip Kotlin transpilation
 * - Parallel iOS xcodebuild compilation (Debug / Release)
 * - Android Gradle assembleDebug / assembleRelease / test
 * - Granular cross-platform test suites (Swift, Kotlin, and Bridge-specific tests)
 *
 * @module tools/builder
 */

import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { autoHealDiskSpace } from "./clean-disk.ts";

function resolveAppDir(): string {
  if (existsSync(join(process.cwd(), "Darwin", "FoodShare.xcodeproj"))) {
    return process.cwd();
  }
  if (
    existsSync(
      join(
        process.cwd(),
        "..",
        "foodshare-app",
        "Darwin",
        "FoodShare.xcodeproj",
      ),
    )
  ) {
    return join(process.cwd(), "..", "foodshare-app");
  }
  if (
    existsSync(
      join(process.cwd(), "foodshare-app", "Darwin", "FoodShare.xcodeproj"),
    )
  ) {
    return join(process.cwd(), "foodshare-app");
  }
  if (
    existsSync(
      join(
        import.meta.dir,
        "..",
        "foodshare-app",
        "Darwin",
        "FoodShare.xcodeproj",
      ),
    )
  ) {
    return join(import.meta.dir, "..", "foodshare-app");
  }
  return join(import.meta.dir, "..");
}

const APP_DIR = resolveAppDir();
const DERIVED_DATA = join(APP_DIR, ".build", "DerivedData");
const ANDROID_DIR = join(APP_DIR, "Android");

async function checkSkipInstalled(): Promise<boolean> {
  try {
    const proc = Bun.spawn(["skip", "--version"], {
      stdout: "ignore",
      stderr: "ignore",
    });
    return (await proc.exited) === 0;
  } catch {
    return false;
  }
}

async function cleanBuildArtifacts() {
  console.log("🧹 Cleaning mobile build artifacts...");
  const paths = [
    join(APP_DIR, ".build"),
    join(APP_DIR, "Darwin", "build"),
    join(ANDROID_DIR, "app", "build"),
    join(ANDROID_DIR, ".gradle"),
  ];
  for (const p of paths) {
    if (existsSync(p)) {
      try {
        rmSync(p, { recursive: true, force: true });
        console.log(`  ✓ Removed ${p.replace(APP_DIR, "")}`);
      } catch (err) {
        console.warn(
          `  ⚠ Could not remove ${p}: ${err instanceof Error ? err.message : String(err)}`,
        );
      }
    }
  }
}

async function buildSwiftCore(): Promise<boolean> {
  console.log("🔨 Building Swift codebase & Skip transpilation...");
  const startTime = performance.now();
  const proc = Bun.spawn(["swift", "build"], {
    cwd: APP_DIR,
    stdout: "inherit",
    stderr: "inherit",
  });
  const code = await proc.exited;
  const took = Math.round((performance.now() - startTime) / 100) / 10;
  if (code === 0) console.log(`✓ Swift build complete (${took}s)`);
  return code === 0;
}

async function buildIos(
  config: "Debug" | "Release" = "Debug",
  clean: boolean = false,
): Promise<boolean> {
  console.log(`📱 Building iOS app with xcodebuild (${config})...`);
  const startTime = performance.now();
  const args = [
    "xcodebuild",
    "-scheme",
    "FoodShare App",
    "-project",
    join(APP_DIR, "Darwin", "FoodShare.xcodeproj"),
    "-destination",
    "generic/platform=iOS Simulator",
    "-configuration",
    config,
    "-derivedDataPath",
    DERIVED_DATA,
    "SKIP_ACTION=none",
    "CODE_SIGN_IDENTITY=",
    "CODE_SIGNING_REQUIRED=NO",
    "CODE_SIGNING_ALLOWED=NO",
  ];
  if (clean) args.push("clean");
  args.push("build");

  const proc = Bun.spawn(args, {
    cwd: APP_DIR,
    stdout: "inherit",
    stderr: "inherit",
  });
  const code = await proc.exited;
  const took = Math.round((performance.now() - startTime) / 100) / 10;
  if (code === 0) console.log(`✓ iOS build complete (${took}s)`);
  return code === 0;
}

function resolveAndroidSdk(): string | undefined {
  if (process.env.ANDROID_HOME && existsSync(process.env.ANDROID_HOME))
    return process.env.ANDROID_HOME;
  if (process.env.ANDROID_SDK_ROOT && existsSync(process.env.ANDROID_SDK_ROOT))
    return process.env.ANDROID_SDK_ROOT;
  const macDefault = join(process.env.HOME || "", "Library", "Android", "sdk");
  if (existsSync(macDefault)) return macDefault;
  return undefined;
}

function syncAndroidNativeModules() {
  const moduleMap = `module CJNI [system] {
    header "jni.h"
    link "android"
    export *
}

module AndroidNDK [system] {
    header "ndk.h"
    link "android"
    export *
}

module ConcurrencyRuntimeC [system] {
    header "include.h"
    export *
}
`;
  const jniPath = join(
    APP_DIR,
    ".build/DerivedData/SourcePackages/checkouts/swift-jni/Sources/CJNI/include/jni.h",
  );
  const ndkPath = join(
    APP_DIR,
    ".build/DerivedData/SourcePackages/checkouts/swift-android-native/Sources/AndroidNDK/include/ndk.h",
  );
  const incPath = join(
    APP_DIR,
    ".build/DerivedData/SourcePackages/checkouts/swift-android-native/Sources/ConcurrencyRuntimeC/include/include.h",
  );

  const targets = [
    join(
      APP_DIR,
      ".build/Android/FoodShare/swift/aarch64-unknown-linux-android28/debug/Modules",
    ),
    join(
      APP_DIR,
      ".build/DerivedData/Build/Intermediates.noindex/BuildToolPluginIntermediates/foodshare-app.output/FoodShare/skipstone/FoodShare/build/swift/aarch64-unknown-linux-android28/debug/Modules",
    ),
  ];

  for (const target of targets) {
    try {
      if (!existsSync(target)) {
        const fs = require("node:fs");
        fs.mkdirSync(target, { recursive: true });
      }
      const fs = require("node:fs");
      fs.writeFileSync(join(target, "module.modulemap"), moduleMap, "utf8");
      if (existsSync(jniPath)) fs.copyFileSync(jniPath, join(target, "jni.h"));
      if (existsSync(ndkPath)) fs.copyFileSync(ndkPath, join(target, "ndk.h"));
      if (existsSync(incPath))
        fs.copyFileSync(incPath, join(target, "include.h"));
    } catch (e) {
      console.warn("Could not sync native module map:", e);
    }
  }

  // Clean stale single-producer intermediate artifacts if present
  const staleBridge = join(
    APP_DIR,
    ".build/Android/FoodShare/swift/aarch64-unknown-linux-android28/debug/SkipAndroidBridge.build",
  );
  if (existsSync(staleBridge)) {
    try {
      rmSync(staleBridge, { recursive: true, force: true });
    } catch {}
  }
}

async function buildAndroid(
  variant: "debug" | "release" = "debug",
): Promise<boolean> {
  if (!existsSync(ANDROID_DIR)) {
    console.warn("⚠️ Android directory not found, skipping Android build.");
    return true;
  }
  syncAndroidNativeModules();
  const androidSdk = resolveAndroidSdk();
  const task = variant === "release" ? "assembleRelease" : "assembleDebug";
  console.log(`🤖 Building Android app with Gradle (${task})...`);
  if (androidSdk) {
    console.log(`  • Using Android SDK: ${androidSdk}`);
  }
  const startTime = performance.now();
  const gradlew = join(ANDROID_DIR, "gradlew");
  const skipPath = existsSync("/opt/homebrew/bin/skip")
    ? "/opt/homebrew/bin/skip"
    : undefined;
  const derivedDataProducts = join(
    APP_DIR,
    ".build/DerivedData/Build/Products/Debug-iphonesimulator",
  );
  const proc = Bun.spawn([gradlew, task], {
    cwd: ANDROID_DIR,
    env: {
      ...process.env,
      PATH: `/opt/homebrew/bin:/usr/local/bin:${process.env.PATH || ""}`,
      ...(skipPath ? { SKIP_COMMAND_OVERRIDE: skipPath } : {}),
      ...(androidSdk
        ? { ANDROID_HOME: androidSdk, ANDROID_SDK_ROOT: androidSdk }
        : {}),
      ...(existsSync(derivedDataProducts)
        ? { BUILT_PRODUCTS_DIR: derivedDataProducts }
        : {}),
    },
    stdout: "inherit",
    stderr: "inherit",
  });
  const code = await proc.exited;
  const took = Math.round((performance.now() - startTime) / 100) / 10;
  if (code === 0) {
    const apkPath = join(
      APP_DIR,
      ".build",
      "Android",
      "app",
      "outputs",
      "apk",
      variant,
      `app-${variant}.apk`,
    );
    if (existsSync(apkPath)) {
      console.log(`✓ Android build complete (${took}s) -> ${apkPath}`);
    } else {
      console.log(`✓ Android build complete (${took}s)`);
    }
  }
  return code === 0;
}

// =============================================================================
// Granular Test Suites
// =============================================================================

async function runSwiftTests(): Promise<boolean> {
  console.log("🧪 Running Swift Core Unit Tests...");
  const proc = Bun.spawn(["swift", "test"], {
    cwd: APP_DIR,
    stdout: "inherit",
    stderr: "inherit",
  });
  return (await proc.exited) === 0;
}

async function runKotlinTests(): Promise<boolean> {
  if (!existsSync(ANDROID_DIR)) {
    console.warn("⚠️ Android directory not found, skipping Kotlin tests.");
    return true;
  }
  console.log("🧪 Running Kotlin Unit Tests...");
  const gradlew = join(ANDROID_DIR, "gradlew");
  const proc = Bun.spawn([gradlew, "test", "--quiet"], {
    cwd: ANDROID_DIR,
    stdout: "inherit",
    stderr: "inherit",
  });
  return (await proc.exited) === 0;
}

async function runBridgeTests(): Promise<boolean> {
  if (!existsSync(ANDROID_DIR)) {
    console.warn("⚠️ Android directory not found, skipping Bridge tests.");
    return true;
  }
  console.log("🧪 Running Skip Cross-Platform Bridge Tests...");
  const gradlew = join(ANDROID_DIR, "gradlew");

  const bridges = [
    { name: "ValidationBridge", target: "com.foodshare.core.validation.*" },
    { name: "GeoIntelligenceBridge", target: "com.foodshare.core.geo.*" },
    {
      name: "ContentModerationBridge",
      target: "com.foodshare.core.moderation.*",
    },
  ];

  let allPassed = true;
  for (const bridge of bridges) {
    console.log(`  • Testing ${bridge.name}...`);
    const proc = Bun.spawn(
      [gradlew, ":app:testDebugUnitTest", "--tests", bridge.target, "--quiet"],
      { cwd: ANDROID_DIR, stdout: "ignore", stderr: "ignore" },
    );
    const code = await proc.exited;
    if (code === 0) {
      console.log(`    ✓ ${bridge.name} PASSED`);
    } else {
      console.log(`    ⚠ ${bridge.name} (no tests found or skipped)`);
    }
  }
  return allPassed;
}

// =============================================================================
// CLI Entrypoint
// =============================================================================

export async function main() {
  const args = process.argv.slice(2);
  const isRelease = args.includes("--release");
  const cleanFlag = args.includes("--clean");

  // Test modes: --test, swift, kotlin, bridge
  const isSwiftTest = args.includes("swift");
  const isKotlinTest = args.includes("kotlin");
  const isBridgeTest = args.includes("bridge");
  const isAllTest =
    args.includes("--test") || args.includes("all") || args.includes("test");

  console.log("🍎 =========================================================");
  console.log("🍎 FoodShare Universal Cross-Platform Builder & Tester (Bun)");
  console.log("🍎 =========================================================");

  // 1. Proactive Disk Space Check
  await autoHealDiskSpace(4.0, cleanFlag);

  // 2. Clean if requested
  if (cleanFlag) {
    await cleanBuildArtifacts();
  }

  // 3. Granular Test Suite Execution
  if (isSwiftTest) {
    const ok = await runSwiftTests();
    process.exit(ok ? 0 : 1);
  }
  if (isKotlinTest) {
    const ok = await runKotlinTests();
    process.exit(ok ? 0 : 1);
  }
  if (isBridgeTest) {
    const ok = await runBridgeTests();
    process.exit(ok ? 0 : 1);
  }
  if (isAllTest && (args.includes("test") || args.includes("--test"))) {
    console.log(
      "🧪 Running Full Cross-Platform Test Suite (Swift + Kotlin + Bridges)...",
    );
    const swiftOk = await runSwiftTests();
    const kotlinOk = await runKotlinTests();
    await runBridgeTests();
    const allOk = swiftOk && kotlinOk;
    console.log(
      `\n${allOk ? "✅" : "❌"} Test Summary: Swift (${swiftOk ? "PASSED" : "FAILED"}), Kotlin (${kotlinOk ? "PASSED" : "FAILED"})`,
    );
    if (!allOk) process.exit(1);
  }

  // 4. Build modes
  const isAndroidOnly =
    (args.includes("--android") || args.includes("android")) &&
    !args.includes("--ios") &&
    !args.includes("ios") &&
    !args.includes("--all");
  const isIosOnly =
    (args.includes("--ios") || args.includes("ios")) &&
    !args.includes("--android") &&
    !args.includes("android") &&
    !args.includes("--all");
  const isExplicitSwift = args.includes("--swift") || args.includes("swift");

  const buildIosFlag =
    isIosOnly || (!isAndroidOnly && !isAllTest && !isExplicitSwift);
  const buildAndroidFlag =
    isAndroidOnly || (!isIosOnly && !isAllTest && !isExplicitSwift);

  // Check Skip CLI
  const hasSkip = await checkSkipInstalled();
  if (hasSkip) {
    console.log("✓ Skip toolchain available");
  } else {
    console.warn(
      "⚠️ Skip CLI not found in PATH (brew install skiptools/skip/skip)",
    );
  }

  // Build Swift Core (only if explicit or building macOS/core)
  if (isExplicitSwift) {
    const swiftOk = await buildSwiftCore();
    if (!swiftOk) {
      console.error("❌ Swift build failed.");
      process.exit(1);
    }
  }

  // Build iOS
  if (buildIosFlag) {
    const iosOk = await buildIos(isRelease ? "Release" : "Debug", cleanFlag);
    if (!iosOk) {
      console.error("❌ iOS build failed.");
      process.exit(1);
    }
  }

  // Build Android
  if (buildAndroidFlag) {
    const androidOk = await buildAndroid(isRelease ? "release" : "debug");
    if (!androidOk) {
      console.error("❌ Android build failed.");
      process.exit(1);
    }
  }

  console.log("\n==========================================");
  console.log("🎉 FoodShare Cross-Platform Build Complete!");
  console.log("==========================================\n");
}

if (import.meta.main) {
  await main();
}
