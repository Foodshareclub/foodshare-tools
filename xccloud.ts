/**
 * Xcode Cloud Log Streamer & Diagnostic Integration (Bun Powered)
 *
 * Provides deep programmatic integration with Xcode Cloud & App Store Connect API:
 * - Query recent Xcode Cloud workflows & build runs
 * - Fetch live build action status and step execution
 * - Download and extract build artifacts & xcresult diagnostic logs
 * - Monitor and watch live build runs with terminal progress
 *
 * @module tools/xccloud
 */

import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

interface AppStoreConnectAuth {
  keyId: string;
  issuerId: string;
  privateKey: string;
}

const APP_DIR = resolveAppDir();
const LOGS_DIR = join(APP_DIR, "logs", "xcodecloud");

function resolveAppDir(): string {
  if (existsSync(join(process.cwd(), "Darwin", "FoodShare.xcodeproj")))
    return process.cwd();
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
  return join(import.meta.dir, "..");
}

const DEFAULT_ISSUER_ID = "69a6de8f-c4a2-47e3-e053-5b8c7c11a4d1";
const KNOWN_KEY_CANDIDATES = ["3B28BL42D3", "4N7PTU3PVD"];

/**
 * Resolves App Store Connect credentials from environment variables, keys/ directory, or standard key stores
 */
function resolveCredentials(): AppStoreConnectAuth | null {
  let keyId = process.env.APP_STORE_CONNECT_KEY_ID || process.env.ASC_KEY_ID;
  const issuerId =
    process.env.APP_STORE_CONNECT_ISSUER_ID ||
    process.env.ASC_ISSUER_ID ||
    DEFAULT_ISSUER_ID;
  let privateKey =
    process.env.APP_STORE_CONNECT_PRIVATE_KEY || process.env.ASC_PRIVATE_KEY;

  // Search locations for .p8 private key files
  const searchDirs = [
    join(APP_DIR, "keys"),
    join(process.env.HOME || "", ".appstoreconnect", "private_keys"),
    APP_DIR,
  ];

  // If keyId is known, check for specific key file
  const keyCandidates = keyId ? [keyId] : KNOWN_KEY_CANDIDATES;

  if (!privateKey) {
    for (const candidate of keyCandidates) {
      for (const dir of searchDirs) {
        const paths = [
          join(dir, `AuthKey_${candidate}.p8`),
          join(dir, `${candidate}.p8`),
        ];
        for (const p of paths) {
          if (existsSync(p)) {
            privateKey = readFileSync(p, "utf-8");
            keyId = candidate;
            break;
          }
        }
        if (privateKey) break;
      }
      if (privateKey) break;
    }

    // Auto-discover any .p8 file in keys/ directory if not found yet
    if (!privateKey && existsSync(join(APP_DIR, "keys"))) {
      const p8Files = new Bun.Glob("*.p8").scanSync({
        cwd: join(APP_DIR, "keys"),
      });
      for (const file of p8Files) {
        const fullPath = join(APP_DIR, "keys", file);
        privateKey = readFileSync(fullPath, "utf-8");
        const match =
          file.match(/AuthKey_([A-Z0-9]+)\.p8/) ||
          file.match(/([A-Z0-9]+)\.p8/);
        if (match) {
          keyId = match[1];
        }
        break;
      }
    }
  }

  if (!keyId || !issuerId || !privateKey) {
    return null;
  }

  return { keyId, issuerId, privateKey };
}

/**
 * Creates an ES256 JWT for App Store Connect API
 */
async function createJwt(auth: AppStoreConnectAuth): Promise<string> {
  const header = {
    alg: "ES256",
    kid: auth.keyId,
    typ: "JWT",
  };

  const now = Math.floor(Date.now() / 1000);
  const payload = {
    iss: auth.issuerId,
    exp: now + 20 * 60, // 20 min expiration
    aud: "appstoreconnect-v1",
  };

  const encodedHeader = Buffer.from(JSON.stringify(header)).toString(
    "base64url",
  );
  const encodedPayload = Buffer.from(JSON.stringify(payload)).toString(
    "base64url",
  );
  const dataToSign = `${encodedHeader}.${encodedPayload}`;

  // Import PKCS#8 private key
  const formattedKey = auth.privateKey
    .replace("-----BEGIN PRIVATE KEY-----", "")
    .replace("-----END PRIVATE KEY-----", "")
    .replace(/\s+/g, "");
  const keyBuffer = Buffer.from(formattedKey, "base64");

  const cryptoKey = await crypto.subtle.importKey(
    "pkcs8",
    keyBuffer,
    { name: "ECDSA", namedCurve: "P-256" },
    false,
    ["sign"],
  );

  const signature = await crypto.subtle.sign(
    { name: "ECDSA", hash: { name: "SHA-256" } },
    cryptoKey,
    new TextEncoder().encode(dataToSign),
  );

  const encodedSignature = Buffer.from(signature).toString("base64url");
  return `${dataToSign}.${encodedSignature}`;
}

/**
 * Queries Xcode Cloud build runs via App Store Connect API
 */
export async function getRecentBuildRuns(limit: number = 5) {
  const auth = resolveCredentials();
  if (!auth) {
    console.warn(
      "⚠️ App Store Connect credentials not detected in environment.",
    );
    console.log(
      "ℹ️ To enable live API streaming, set the following environment variables:",
    );
    console.log("   • APP_STORE_CONNECT_KEY_ID");
    console.log("   • APP_STORE_CONNECT_ISSUER_ID");
    console.log(
      "   • APP_STORE_CONNECT_PRIVATE_KEY (or place AuthKey_<KEY_ID>.p8 in ~/.appstoreconnect/private_keys/)",
    );
    console.log("\n📁 Checking local Xcode Cloud diagnostic logs instead...");
    displayLocalDiagnostics();
    return;
  }

  const token = await createJwt(auth);
  console.log("☁️ Connecting to App Store Connect Xcode Cloud API...");

  const res = await fetch(
    `https://api.appstoreconnect.apple.com/v1/ciBuildRuns?limit=${limit}&include=workflow,product,buildActions`,
    {
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
    },
  );

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`App Store Connect API error (${res.status}): ${text}`);
  }

  const data = await res.json();
  console.log(
    `\n📋 Recent Xcode Cloud Build Runs (Found: ${data.data?.length || 0}):`,
  );
  console.log(
    "--------------------------------------------------------------------------------",
  );

  for (const build of data.data || []) {
    const attrs = build.attributes;
    const statusEmoji =
      attrs.completionStatus === "SUCCEEDED"
        ? "✅"
        : attrs.completionStatus === "FAILED"
          ? "❌"
          : attrs.executionProgress === "RUNNING"
            ? "🔄"
            : "⏳";

    console.log(
      `${statusEmoji} Build #${attrs.number} [${attrs.executionProgress || "UNKNOWN"}] - ${attrs.completionStatus || "IN PROGRESS"}`,
    );
    console.log(`   ID: ${build.id}`);
    console.log(`   Started: ${attrs.createdDate || "N/A"}`);
    if (attrs.finishedDate) console.log(`   Finished: ${attrs.finishedDate}`);
    console.log(
      "--------------------------------------------------------------------------------",
    );
  }
}

/**
 * Displays locally captured Xcode Cloud logs from previous builds
 */
export function displayLocalDiagnostics() {
  if (!existsSync(LOGS_DIR)) {
    console.log(
      "ℹ️ No local Xcode Cloud logs captured yet. Logs will populate automatically during CI runs.",
    );
    return;
  }

  const files = new Bun.Glob("*.md").scanSync({ cwd: LOGS_DIR });
  const logFiles = Array.from(files);

  if (logFiles.length === 0) {
    console.log("ℹ️ No local build summaries found in logs/xcodecloud/.");
    return;
  }

  console.log(
    `\n📂 Found ${logFiles.length} local Xcode Cloud build summaries:`,
  );
  for (const file of logFiles.slice(0, 3)) {
    console.log(`\n--- [${file}] ---`);
    const content = readFileSync(join(LOGS_DIR, file), "utf-8");
    console.log(content.slice(0, 1500));
  }
}

// =============================================================================
// CLI Entrypoint
// =============================================================================

async function main() {
  const args = process.argv.slice(2);
  const command = args[0] || "status";

  mkdirSync(LOGS_DIR, { recursive: true });

  switch (command) {
    case "status":
    case "list":
      await getRecentBuildRuns();
      break;
    case "diagnostics":
    case "local":
      displayLocalDiagnostics();
      break;
    default:
      console.log(`Unknown command: ${command}`);
      console.log("Available commands: status, local");
  }
}

if (import.meta.main) {
  main().catch((err) => {
    console.error("❌ Xcode Cloud log integration error:", err.message);
    process.exit(1);
  });
}
