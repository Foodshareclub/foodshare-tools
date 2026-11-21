#!/usr/bin/env bun
// Report the storage that artifacts and Actions caches occupy, across the
// repositories that share this organisation's Actions allowance.
//
// The storage guard in this repository stops new leaks. It cannot say what the
// repositories are already holding, and it cannot see the two shapes that are
// only visible from the outside:
//
//   1. Duplicate caches. The same key saved more than once (typically on
//      `main` and on a pull request merge ref) is a second copy that only one
//      of the entries can ever be restored.
//   2. Caches whose pull request is closed or merged. A cache written under
//      `refs/pull/<n>/merge` is unreachable once that ref stops running, so
//      every byte left there is reclaimable.
//
// It also watches for the failure the quota itself produces: an
// `actions/upload-artifact` step that dies with the storage-quota message.
// That failure is a red step on one run and is easy to miss; here it fails
// the scheduled report, which is the alert.
//
// Usage: GITHUB_TOKEN=... bun scripts/storage-report.ts
// Env:   STORAGE_REPOS       comma-separated owner/name list
//        STORAGE_REPORT_DAYS how far back to scan failed runs (default 7)
//        GITHUB_API_URL      API base (default https://api.github.com)

import { mkdtempSync, rmSync, writeFileSync } from "bun:fs";
import { tmpdir } from "bun:os";
import { join } from "bun:path";

const API = (process.env.GITHUB_API_URL ?? "https://api.github.com").replace(/\/+$/, "");
const TOKEN = process.env.GITHUB_TOKEN ?? process.env.GH_TOKEN ?? "";
const DEFAULT_REPOS = [
  "Foodshareclub/foodshare-tools",
  "Foodshareclub/foodshare-web",
  "Foodshareclub/foodshare-app",
  "Foodshareclub/foodshare-backend",
];
const REPOS = (process.env.STORAGE_REPOS ?? DEFAULT_REPOS.join(","))
  .split(",")
  .map((repo) => repo.trim())
  .filter((repo) => repo !== "");
const REPORT_DAYS = Number.parseInt(process.env.STORAGE_REPORT_DAYS ?? "7", 10) || 7;

/** Cap the number of job logs fetched, so a bad day cannot stall the report. */
const QUOTA_LOG_LIMIT = 40;

export type CacheEntry = {
  id: number;
  ref: string;
  key: string;
  size_in_bytes: number;
  created_at: string;
  last_accessed_at: string;
};

export type ArtifactEntry = {
  id: number;
  name: string;
  size_in_bytes: number;
  expired: boolean;
};

export type DuplicateGroup = {
  key: string;
  entries: CacheEntry[];
  totalBytes: number;
  wastedBytes: number;
};

export type PullState = { state: string; merged: boolean };

export type QuotaFailure = {
  repo: string;
  run: string;
  job: string;
  step: string;
  line: string;
};

export type RepoReport = {
  repo: string;
  artifactBytes: number;
  artifactCount: number;
  cacheBytes: number;
  cacheCount: number;
  duplicates: DuplicateGroup[];
  closedPr: CacheEntry[];
};

function headers(): Record<string, string> {
  return {
    authorization: `Bearer ${TOKEN}`,
    accept: "application/vnd.github+json",
    "user-agent": "foodshare-storage-report",
    "x-github-api-version": "2022-11-28",
  };
}

async function request<T>(path: string): Promise<T> {
  const response = await fetch(`${API}${path}`, { headers: headers() });
  if (!response.ok) {
    const body = (await response.text()).slice(0, 300);
    throw new Error(`GET ${path} -> ${response.status}: ${body}`);
  }
  return (await response.json()) as T;
}

async function paginate<T>(path: string, field: string): Promise<T[]> {
  const all: T[] = [];
  for (let page = 1; page <= 50; page += 1) {
    const separator = path.includes("?") ? "&" : "?";
    const body = await request<Record<string, T[]>>(`${path}${separator}per_page=100&page=${page}`);
    const batch = body[field] ?? [];
    all.push(...batch);
    if (batch.length < 100) {
      return all;
    }
  }
  throw new Error(`${path}: more than 5000 entries; refusing to guess`);
}

/** Group caches that share a key. One entry can be restored; the rest cannot. */
export function duplicateCacheGroups(caches: CacheEntry[]): DuplicateGroup[] {
  const byKey = new Map<string, CacheEntry[]>();
  for (const cache of caches) {
    const entries = byKey.get(cache.key);
    if (entries) {
      entries.push(cache);
    } else {
      byKey.set(cache.key, [cache]);
    }
  }

  return [...byKey.entries()]
    .filter(([, entries]) => entries.length > 1)
    .map(([key, entries]) => {
      const sorted = [...entries].sort((a, b) => b.size_in_bytes - a.size_in_bytes);
      const totalBytes = sorted.reduce((sum, entry) => sum + entry.size_in_bytes, 0);
      return { key, entries: sorted, totalBytes, wastedBytes: totalBytes - sorted[0].size_in_bytes };
    })
    .sort((a, b) => b.wastedBytes - a.wastedBytes);
}

/** Caches on `refs/pull/<n>/merge` whose pull request is no longer open. */
export function closedPrCaches(caches: CacheEntry[], states: Map<string, PullState>): CacheEntry[] {
  return caches.filter((cache) => {
    const match = /^refs\/pull\/(\d+)\/merge$/.exec(cache.ref);
    if (!match) {
      return false;
    }
    const state = states.get(match[1]);
    if (state === undefined) {
      return false;
    }
    return state.state !== "open" || state.merged;
  });
}

const QUOTA_PATTERNS = [
  /artifact storage quota/i,
  /storage quota (?:has been|was) (?:hit|exceeded)/i,
  /quota has been hit/i,
  /no space left on device/i,
];

/** The first line of a job log that reports the storage quota, if any. */
export function quotaLine(log: string): string | undefined {
  for (const line of log.split(/\r?\n/)) {
    if (QUOTA_PATTERNS.some((pattern) => pattern.test(line))) {
      return line.trim().slice(0, 400);
    }
  }
  return undefined;
}

async function pullStates(repo: string, caches: CacheEntry[]): Promise<Map<string, PullState>> {
  const numbers = [
    ...new Set(
      caches
        .map((cache) => /^refs\/pull\/(\d+)\/merge$/.exec(cache.ref)?.[1])
        .filter((number): number is string => number !== undefined),
    ),
  ];

  const states = new Map<string, PullState>();
  for (const number of numbers) {
    try {
      const pull = await request<{ state: string; merged_at: string | null }>(`/repos/${repo}/pulls/${number}`);
      states.set(number, { state: pull.state, merged: pull.merged_at !== null });
    } catch {
      // A deleted pull request is not open either; treat it as closed.
      states.set(number, { state: "missing", merged: false });
    }
  }
  return states;
}

/**
 * Decode job logs. The API serves the log archive as plain text for this
 * endpoint today, but its documentation promises a zip; accept both rather
 * than silently reading zero quota messages from one of them.
 */
export function decodeJobLog(bytes: Uint8Array): string {
  const isZip = bytes.length >= 2 && bytes[0] === 0x50 && bytes[1] === 0x4b;
  if (!isZip) {
    return new TextDecoder().decode(bytes);
  }

  const directory = mkdtempSync(join(tmpdir(), "storage-report-"));
  try {
    const path = join(directory, "job-logs.zip");
    writeFileSync(path, bytes);
    const unzip = Bun.spawnSync(["unzip", "-p", path], { stdout: "pipe", stderr: "pipe" });
    return unzip.exitCode === 0 ? new TextDecoder().decode(unzip.stdout) : "";
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

async function jobLog(repo: string, jobId: number): Promise<string> {
  const response = await fetch(`${API}/repos/${repo}/actions/jobs/${jobId}/logs`, {
    headers: headers(),
    redirect: "manual",
  });
  const location = response.headers.get("location");
  if (!location) {
    // Some API responses serve the log body directly instead of redirecting.
    return response.ok ? decodeJobLog(new Uint8Array(await response.arrayBuffer())) : "";
  }

  // The redirect target is signed blob storage. The token belongs to the API
  // request, not to that URL.
  const download = await fetch(location);
  if (!download.ok) {
    return "";
  }

  return decodeJobLog(new Uint8Array(await download.arrayBuffer()));
}

async function quotaFailures(repo: string, since: string): Promise<QuotaFailure[]> {
  const runs = await request<{ workflow_runs: Array<{ id: number; html_url: string }> }>(
    `/repos/${repo}/actions/runs?status=failure&created=${encodeURIComponent(`>=${since}`)}&per_page=30`,
  );

  const failures: QuotaFailure[] = [];
  let downloads = 0;
  for (const run of runs.workflow_runs) {
    const jobs = await request<{
      jobs: Array<{ id: number; name: string; steps?: Array<{ name: string; conclusion: string | null }> }>;
    }>(`/repos/${repo}/actions/runs/${run.id}/jobs?per_page=100`);

    for (const job of jobs.jobs) {
      const failedUpload = job.steps?.find(
        (step) => step.conclusion === "failure" && /upload|artifact/i.test(step.name),
      );
      if (failedUpload === undefined) {
        continue;
      }
      if (downloads >= QUOTA_LOG_LIMIT) {
        return failures;
      }
      downloads += 1;
      const line = quotaLine(await jobLog(repo, job.id));
      if (line !== undefined) {
        failures.push({ repo, run: run.html_url, job: job.name, step: failedUpload.name, line });
      }
    }
  }
  return failures;
}

async function repoReport(repo: string): Promise<RepoReport> {
  const usage = await request<{ active_caches_size_in_bytes: number }>(`/repos/${repo}/actions/cache/usage`);
  const caches = await paginate<CacheEntry>(`/repos/${repo}/actions/caches`, "actions_caches");
  const artifacts = await paginate<ArtifactEntry>(`/repos/${repo}/actions/artifacts`, "artifacts");
  const activeArtifacts = artifacts.filter((artifact) => !artifact.expired);
  const states = await pullStates(repo, caches);

  return {
    repo,
    artifactBytes: activeArtifacts.reduce((sum, artifact) => sum + artifact.size_in_bytes, 0),
    artifactCount: activeArtifacts.length,
    cacheBytes: usage.active_caches_size_in_bytes,
    cacheCount: caches.length,
    duplicates: duplicateCacheGroups(caches),
    closedPr: closedPrCaches(caches, states),
  };
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}

export function renderReport(reports: RepoReport[], failures: QuotaFailure[], generatedAt: string): string {
  const lines: string[] = ["# Actions storage report", "", `Generated ${generatedAt}.`, ""];

  lines.push(
    "## Totals",
    "",
    "| Repository | Artifacts (active) | Artifact count | Caches | Cache count | Total |",
    "|---|---:|---:|---:|---:|---:|",
  );
  for (const report of reports) {
    lines.push(
      `| ${report.repo} | ${formatBytes(report.artifactBytes)} | ${report.artifactCount} | ${formatBytes(
        report.cacheBytes,
      )} | ${report.cacheCount} | ${formatBytes(report.artifactBytes + report.cacheBytes)} |`,
    );
  }
  const artifactBytes = reports.reduce((sum, report) => sum + report.artifactBytes, 0);
  const artifactCount = reports.reduce((sum, report) => sum + report.artifactCount, 0);
  const cacheBytes = reports.reduce((sum, report) => sum + report.cacheBytes, 0);
  const cacheCount = reports.reduce((sum, report) => sum + report.cacheCount, 0);
  lines.push(
    `| **All repositories** | ${formatBytes(artifactBytes)} | ${artifactCount} | ${formatBytes(
      cacheBytes,
    )} | ${cacheCount} | ${formatBytes(artifactBytes + cacheBytes)} |`,
    "",
  );

  lines.push("## Duplicate caches", "");
  const duplicates = reports.flatMap((report) => report.duplicates.map((group) => ({ report, group })));
  if (duplicates.length === 0) {
    lines.push("None.", "");
  } else {
    lines.push("| Repository | Key | Entries | Total | Wasted | Refs |", "|---|---|---:|---:|---:|---|");
    for (const { report, group } of duplicates) {
      const refs = group.entries.map((entry) => entry.ref).join("<br>");
      lines.push(
        `| ${report.repo} | \`${group.key}\` | ${group.entries.length} | ${formatBytes(
          group.totalBytes,
        )} | ${formatBytes(group.wastedBytes)} | ${refs} |`,
      );
    }
    lines.push("");
  }

  lines.push("## Caches whose pull request is closed", "");
  const closed = reports.flatMap((report) => report.closedPr.map((cache) => ({ report, cache })));
  if (closed.length === 0) {
    lines.push("None.", "");
  } else {
    lines.push("| Repository | Ref | Size | Pull request |", "|---|---|---:|---|");
    for (const { report, cache } of closed) {
      const number = /^refs\/pull\/(\d+)\/merge$/.exec(cache.ref)?.[1] ?? "";
      lines.push(
        `| ${report.repo} | ${cache.ref} | ${formatBytes(cache.size_in_bytes)} | [#${number}](https://github.com/${
          report.repo
        }/pull/${number}) |`,
      );
    }
    lines.push("");
  }

  lines.push("## Quota failures", "");
  if (failures.length === 0) {
    lines.push(`None in the last ${REPORT_DAYS} days.`, "");
  } else {
    lines.push("| Repository | Run | Job | Step | Line |", "|---|---|---|---|---|");
    for (const failure of failures) {
      lines.push(
        `| ${failure.repo} | [run](${failure.run}) | ${failure.job} | ${failure.step} | \`${failure.line.replace(
          /\|/g,
          "\\|",
        )}\` |`,
      );
    }
    lines.push("");
  }

  return lines.join("\n");
}

if (import.meta.main) {
  if (TOKEN === "") {
    console.error("❌ GITHUB_TOKEN (or GH_TOKEN) is required to read Actions storage");
    process.exit(2);
  }

  const generatedAt = new Date().toISOString();
  const since = new Date(Date.now() - REPORT_DAYS * 86_400_000).toISOString();

  const reports: RepoReport[] = [];
  for (const repo of REPOS) {
    reports.push(await repoReport(repo));
  }

  const failures: QuotaFailure[] = [];
  for (const repo of REPOS) {
    failures.push(...(await quotaFailures(repo, since)));
  }

  const markdown = renderReport(reports, failures, generatedAt);
  const summary = process.env.GITHUB_STEP_SUMMARY;
  if (summary) {
    await Bun.write(summary, `${markdown}\n`);
  } else {
    console.log(markdown);
  }

  for (const failure of failures) {
    const message = `${failure.repo} ${failure.job} › ${failure.step}: ${failure.line} (${failure.run})`;
    console.log(`::error title=Actions artifact quota::${message}`);
  }

  if (failures.length > 0) {
    process.exit(1);
  }
}
