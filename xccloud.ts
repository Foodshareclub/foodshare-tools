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
  if (existsSync(join(process.cwd(), "Darwin", "FoodShare.xcodeproj"))) {
    return process.cwd();
  }
  if (
    existsSync(
      join(process.cwd(), "foodshare-app", "Darwin", "FoodShare.xcodeproj"),
    )
  ) {
    return join(process.cwd(), "foodshare-app");
  }
  return join(import.meta.dir, "..");
}

const DEFAULT_ISSUER_ID = "69a6de8f-c4a2-47e3-e053-5b8c7c11a4d1";
const KNOWN_KEY_CANDIDATES = ["3B28BL42D3", "4N7PTU3PVD"];

/**
 * Resolves App Store Connect credentials from environment variables, keys/ directory, or standard key stores
 */
export function resolveCredentials(): AppStoreConnectAuth | null {
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
export async function createJwt(auth: AppStoreConnectAuth): Promise<string> {
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

  // 1. Fetch Xcode Cloud Products
  const productsRes = await fetch(
    "https://api.appstoreconnect.apple.com/v1/ciProducts?include=app,bundleId",
    {
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
    },
  );

  if (!productsRes.ok) {
    const text = await productsRes.text();
    throw new Error(
      `App Store Connect API error (${productsRes.status}): ${text}`,
    );
  }

  const productsData = await productsRes.json();
  const products = productsData.data || [];

  if (products.length === 0) {
    console.log(
      "ℹ️ No Xcode Cloud products found for this App Store Connect team.",
    );
    return;
  }

  console.log(
    `📦 Found ${products.length} Xcode Cloud Product(s): ${products.map((p: any) => p.attributes?.name).join(", ")}`,
  );

  for (const product of products) {
    if (
      product.attributes?.name &&
      !product.attributes.name.toLowerCase().includes("foodshare")
    ) {
      continue; // Focus on FoodShare by default
    }

    console.log(
      `\n🔍 Fetching recent build runs for: ${product.attributes?.name}...`,
    );
    const buildRunsRes = await fetch(
      `https://api.appstoreconnect.apple.com/v1/ciProducts/${product.id}/buildRuns?limit=${limit}&include=workflow&sort=-number`,
      {
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
      },
    );

    if (!buildRunsRes.ok) {
      const text = await buildRunsRes.text();
      console.warn(
        `  ⚠️ Failed to fetch build runs for ${product.attributes?.name}: ${text}`,
      );
      continue;
    }

    const buildData = await buildRunsRes.json();
    const builds = buildData.data || [];
    const included = buildData.included || [];

    console.log(
      `\n📋 Recent Xcode Cloud Build Runs (Found: ${builds.length}):`,
    );
    console.log(
      "================================================================================",
    );

    for (const build of builds) {
      const attrs = build.attributes;
      const workflowId = build.relationships?.workflow?.data?.id;
      const workflow = included.find(
        (i: any) => i.type === "ciWorkflows" && i.id === workflowId,
      );
      const workflowName = workflow?.attributes?.name || "Workflow";

      const statusEmoji =
        attrs.completionStatus === "SUCCEEDED"
          ? "✅"
          : attrs.completionStatus === "FAILED" ||
              attrs.completionStatus === "ERRORED"
            ? "❌"
            : attrs.completionStatus === "CANCELED"
              ? "⏹️"
              : attrs.executionProgress === "RUNNING"
                ? "🔄"
                : "⏳";

      console.log(`${statusEmoji} Build #${attrs.number} [${workflowName}]`);
      console.log(
        `   Status: ${attrs.completionStatus || attrs.executionProgress || "IN PROGRESS"}`,
      );
      console.log(`   ID: ${build.id}`);
      if (attrs.sourceCommit?.commitSha)
        console.log(
          `   Commit: ${attrs.sourceCommit.commitSha.slice(0, 7)} ("${attrs.sourceCommit.message?.trim() || ""}")`,
        );
      console.log(`   Started: ${attrs.createdDate || "N/A"}`);
      if (attrs.finishedDate) console.log(`   Finished: ${attrs.finishedDate}`);

      // Fetch build issues/errors if failed
      if (
        attrs.completionStatus === "FAILED" ||
        attrs.completionStatus === "ERRORED"
      ) {
        try {
          const issuesRes = await fetch(
            `https://api.appstoreconnect.apple.com/v1/ciBuildRuns/${build.id}/issues`,
            {
              headers: { Authorization: `Bearer ${token}` },
            },
          );
          if (issuesRes.ok) {
            const issuesData = await issuesRes.json();
            const issues = issuesData.data || [];
            if (issues.length > 0) {
              console.log(`   ⚠️ Issues (${issues.length}):`);
              for (const issue of issues.slice(0, 5)) {
                console.log(
                  `      • [${issue.attributes?.issueType || "Error"}] ${issue.attributes?.message || ""}`,
                );
                if (issue.attributes?.fileSource?.path) {
                  console.log(
                    `        File: ${issue.attributes.fileSource.path}:${issue.attributes.fileSource.lineNumber || ""}`,
                  );
                }
              }
            }
          }
        } catch {
          // Ignore issue fetch error
        }
      }
      console.log(
        "--------------------------------------------------------------------------------",
      );
    }
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

/**
 * Fetches and displays logs and artifacts for a specific Xcode Cloud build run
 */
export async function fetchBuildLogs(requestedBuildId?: string) {
  const auth = resolveCredentials();
  if (!auth) {
    console.warn(
      "⚠️ App Store Connect credentials not detected in environment.",
    );
    return;
  }

  const token = await createJwt(auth);

  let buildId = requestedBuildId;

  // If no buildId passed, get the latest build
  if (!buildId) {
    const productsRes = await fetch(
      "https://api.appstoreconnect.apple.com/v1/ciProducts?include=app",
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    const productsData = await productsRes.json();
    const foodshareProduct = (productsData.data || []).find((p: any) =>
      p.attributes?.name?.toLowerCase().includes("foodshare"),
    );

    if (foodshareProduct) {
      const runsRes = await fetch(
        `https://api.appstoreconnect.apple.com/v1/ciProducts/${foodshareProduct.id}/buildRuns?limit=1&sort=-number`,
        { headers: { Authorization: `Bearer ${token}` } },
      );
      const runsData = await runsRes.json();
      buildId = runsData.data?.[0]?.id;
    }
  }

  if (!buildId) {
    console.error("❌ No build run found.");
    return;
  }

  console.log(`📥 Fetching build actions & logs for Build ID: ${buildId}...`);

  // 1. Fetch Build Actions
  const actionsRes = await fetch(
    `https://api.appstoreconnect.apple.com/v1/ciBuildRuns/${buildId}/actions`,
    {
      headers: { Authorization: `Bearer ${token}` },
    },
  );

  if (!actionsRes.ok) {
    const text = await actionsRes.text();
    throw new Error(
      `Failed to fetch build actions (${actionsRes.status}): ${text}`,
    );
  }

  const actionsData = await actionsRes.json();
  const actions = actionsData.data || [];

  console.log(`📋 Found ${actions.length} Action(s):`);

  for (const action of actions) {
    const attrs = action.attributes;
    console.log(
      `\n🔹 Action: ${attrs.name || attrs.actionType} [${attrs.actionType}]`,
    );
    console.log(
      `   Status: ${attrs.completionStatus || attrs.executionProgress}`,
    );
    if (attrs.startedDate) console.log(`   Started: ${attrs.startedDate}`);
    if (attrs.finishedDate) console.log(`   Finished: ${attrs.finishedDate}`);

    // Fetch action issues
    const issuesRes = await fetch(
      `https://api.appstoreconnect.apple.com/v1/ciBuildActions/${action.id}/issues`,
      { headers: { Authorization: `Bearer ${token}` } },
    );
    if (issuesRes.ok) {
      const issuesData = await issuesRes.json();
      const issues = issuesData.data || [];
      if (issues.length > 0) {
        console.log(`\n   ⚠️ Action Issues (${issues.length}):`);
        for (const issue of issues) {
          console.log(
            `      • [${issue.attributes?.issueType || "Error"}] ${issue.attributes?.message || ""}`,
          );
          if (issue.attributes?.fileSource?.path) {
            console.log(
              `        File: ${issue.attributes.fileSource.path}:${issue.attributes.fileSource.lineNumber || ""}`,
            );
          }
        }
      }
    }

    // Fetch artifacts
    const artifactsRes = await fetch(
      `https://api.appstoreconnect.apple.com/v1/ciBuildActions/${action.id}/artifacts`,
      { headers: { Authorization: `Bearer ${token}` } },
    );

    if (artifactsRes.ok) {
      const artifactsData = await artifactsRes.json();
      const artifacts = artifactsData.data || [];
      if (artifacts.length > 0) {
        console.log(`\n   📦 Artifacts (${artifacts.length}):`);
        for (const artifact of artifacts) {
          console.log(
            `      • ${artifact.attributes?.fileName} (${artifact.attributes?.fileType})`,
          );
          if (artifact.attributes?.downloadUrl) {
            console.log(
              `        Download URL: ${artifact.attributes.downloadUrl}`,
            );
          }
        }
      }
    }
  }
}

/**
 * Monitors and watches an in-progress Xcode Cloud build run until completion
 */
export async function watchBuild(requestedBuildId?: string) {
  const auth = resolveCredentials();
  if (!auth) {
    console.warn("⚠️ App Store Connect credentials not detected.");
    return;
  }

  const token = await createJwt(auth);
  let buildId = requestedBuildId;

  if (!buildId) {
    const productsRes = await fetch(
      "https://api.appstoreconnect.apple.com/v1/ciProducts",
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    const productsData = await productsRes.json();
    const product = (productsData.data || []).find((p: any) =>
      p.attributes?.name?.toLowerCase().includes("foodshare"),
    );
    if (product) {
      const runsRes = await fetch(
        `https://api.appstoreconnect.apple.com/v1/ciProducts/${product.id}/buildRuns?limit=1&sort=-number`,
        { headers: { Authorization: `Bearer ${token}` } },
      );
      const runsData = await runsRes.json();
      buildId = runsData.data?.[0]?.id;
    }
  }

  if (!buildId) {
    console.error("❌ No build run found to watch.");
    return;
  }

  console.log(`📡 Watching Xcode Cloud Build ID: ${buildId}...`);
  let isDone = false;
  let lastProgress = "";

  while (!isDone) {
    const res = await fetch(
      `https://api.appstoreconnect.apple.com/v1/ciBuildRuns/${buildId}`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    if (res.ok) {
      const data = await res.json();
      const attrs = data.data?.attributes;
      const progress = attrs?.executionProgress || "UNKNOWN";
      const status = attrs?.completionStatus || "IN PROGRESS";

      if (progress !== lastProgress || status !== "IN PROGRESS") {
        console.log(
          `[${new Date().toLocaleTimeString()}] 🔄 Build #${attrs?.number} - Progress: ${progress} | Status: ${status}`,
        );
        lastProgress = progress;
      }

      if (
        status !== "IN PROGRESS" &&
        (progress === "COMPLETE" || attrs?.finishedDate)
      ) {
        isDone = true;
        console.log(`\n🏁 Build finished with status: ${status}`);
        if (status === "SUCCEEDED") {
          console.log("🎉 Xcode Cloud build succeeded!");
        } else {
          console.log("❌ Build failed. Fetching action issues...");
          await fetchBuildLogs(buildId);
        }
        break;
      }
    }
    await Bun.sleep(8000);
  }
}

/**
 * Triggers a new Xcode Cloud build run for the primary workflow
 */
export async function triggerBuild(workflowId?: string): Promise<void> {
  const auth = resolveCredentials();
  if (!auth) {
    console.error("❌ Could not resolve App Store Connect credentials.");
    process.exit(1);
  }

  const token = await createJwt(auth);
  let targetWorkflowId = workflowId;

  if (!targetWorkflowId) {
    const productsRes = await fetch(
      "https://api.appstoreconnect.apple.com/v1/ciProducts",
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    if (productsRes.ok) {
      const pData = await productsRes.json();
      const product =
        pData.data?.find((p: any) => p.attributes.name === "FoodShare App") ||
        pData.data?.[0];
      if (product) {
        const wfRes = await fetch(
          `https://api.appstoreconnect.apple.com/v1/ciProducts/${product.id}/workflows`,
          {
            headers: { Authorization: `Bearer ${token}` },
          },
        );
        if (wfRes.ok) {
          const wfData = await wfRes.json();
          targetWorkflowId = wfData.data?.[0]?.id;
        }
      }
    }
  }

  if (!targetWorkflowId) {
    console.error(
      "❌ Could not find an active Xcode Cloud workflow to trigger.",
    );
    process.exit(1);
  }

  console.log(
    `🚀 Triggering new Xcode Cloud build for workflow: ${targetWorkflowId}...`,
  );
  const res = await fetch(
    "https://api.appstoreconnect.apple.com/v1/ciBuildRuns",
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        data: {
          type: "ciBuildRuns",
          relationships: {
            workflow: {
              data: {
                type: "ciWorkflows",
                id: targetWorkflowId,
              },
            },
          },
        },
      }),
    },
  );

  if (!res.ok) {
    const errorText = await res.text();
    console.error(
      `❌ Failed to trigger build run (HTTP ${res.status}):`,
      errorText,
    );
    process.exit(1);
  }

  const data = await res.json();
  const build = data.data;
  console.log(
    `✅ Build #${build.attributes.number} successfully queued! (ID: ${build.id})`,
  );
  console.log(`👀 Starting live watch...\n`);
  await watchBuild(build.id);
}

// =============================================================================
// CLI Entrypoint
// =============================================================================

async function main() {
  const args = process.argv.slice(2);
  const command = args[0] || "status";

  mkdirSync(LOGS_DIR, { recursive: true });

  switch (command) {
    case "jwt":
    case "token": {
      const auth = resolveCredentials();
      if (!auth) {
        console.error("❌ Could not resolve App Store Connect credentials.");
        process.exit(1);
      }
      const token = await createJwt(auth);
      console.log(token);
      break;
    }
    case "status":
    case "list":
      await getRecentBuildRuns();
      break;
    case "start":
    case "trigger":
    case "build":
      await triggerBuild(args[1]);
      break;
    case "watch":
      await watchBuild(args[1]);
      break;
    case "logs":
    case "log":
      await fetchBuildLogs(args[1]);
      break;
    case "diagnostics":
    case "local":
      displayLocalDiagnostics();
      break;
    default:
      console.log(`Unknown command: ${command}`);
      console.log(
        "Available commands: status, start, watch [buildId], logs [buildId], jwt, local",
      );
  }
}

if (import.meta.main) {
  main().catch((err) => {
    console.error("❌ Xcode Cloud log integration error:", err.message);
    process.exit(1);
  });
}
