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
//        BILLING_TOKEN       org billing-read token (defaults to GITHUB_TOKEN);
//                            the cross-repo token may not carry billing read
//        STORAGE_ORG         organisation for billing (default first repo owner)
//        STORAGE_ALLOWANCE_GB_HOURS  monthly artifact allowance (default 372,
//                            which is 500 MB metered over a 744-hour month)

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
const BILLING_TOKEN = process.env.BILLING_TOKEN ?? TOKEN;
const ORG = process.env.STORAGE_ORG ?? REPOS[0]?.split("/")[0] ?? "";
/**
 * 500 MB of artifact storage for a 744-hour billing month. GitHub meters the
 * allowance in GB-hours (0.5 GB x 744 h), not as a 500 MB snapshot; the
 * summary endpoint discounts exactly this much once billable usage reaches it.
 */
const DEFAULT_ALLOWANCE_GB_HOURS = 372;
const ALLOWANCE_GB_HOURS =
  Number.parseFloat(process.env.STORAGE_ALLOWANCE_GB_HOURS ?? `${DEFAULT_ALLOWANCE_GB_HOURS}`) ||
  DEFAULT_ALLOWANCE_GB_HOURS;
const FALLBACK_PRICE_PER_GB_HOUR = 0.25 / 744;

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
  private: boolean;
  artifactBytes: number;
  artifactCount: number;
  cacheBytes: number;
  cacheCount: number;
  duplicates: DuplicateGroup[];
  closedPr: CacheEntry[];
};

export type BillingUsageItem = {
  date: string;
  product: string;
  sku: string;
  quantity: number;
  unitType: string;
  pricePerUnit?: number;
  grossAmount?: number;
  discountAmount?: number;
  netAmount?: number;
  repositoryName?: string;
};

export type BillingSummaryItem = {
  product: string;
  sku: string;
  grossQuantity: number;
  discountQuantity: number;
  netQuantity: number;
  grossAmount: number;
  discountAmount: number;
  netAmount: number;
  pricePerUnit: number;
  unitType: string;
};

export type BillingUsageResponse = { usageItems?: BillingUsageItem[] };
export type BillingSummaryResponse = {
  usageItems?: BillingSummaryItem[];
  timePeriod?: { year: number; month: number };
};

export type QuotaMetrics = {
  month: string;
  day: number;
  daysInMonth: number;
  actionsGbHours: number;
  publicActionsGbHours: number;
  packagesGbHours: number;
  combinedGbHours: number;
  allowanceGbHours: number;
  usedPercent: number;
  netGbHours: number;
  exhausted: boolean;
  exhaustedOn?: string;
  currentArtifactBytes: number;
  currentGb: number;
  projectedGbHours: number;
  projectedOverGbHours: number;
  accruedCost: number;
  projectedCost: number;
  projectedExhaustion?: string;
};

export type QuotaReport = ({ available: true } & QuotaMetrics) | { available: false; error: string };

function headers(token: string): Record<string, string> {
  return {
    authorization: `Bearer ${token}`,
    accept: "application/vnd.github+json",
    "user-agent": "foodshare-storage-report",
    "x-github-api-version": "2022-11-28",
  };
}

async function request<T>(path: string, token = TOKEN): Promise<T> {
  const response = await fetch(`${API}${path}`, { headers: headers(token) });
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
    headers: headers(TOKEN),
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
  const meta = await request<{ private: boolean }>(`/repos/${repo}`);
  const usage = await request<{ active_caches_size_in_bytes: number }>(`/repos/${repo}/actions/cache/usage`);
  const caches = await paginate<CacheEntry>(`/repos/${repo}/actions/caches`, "actions_caches");
  const artifacts = await paginate<ArtifactEntry>(`/repos/${repo}/actions/artifacts`, "artifacts");
  const activeArtifacts = artifacts.filter((artifact) => !artifact.expired);
  const states = await pullStates(repo, caches);

  return {
    repo,
    private: meta.private,
    artifactBytes: activeArtifacts.reduce((sum, artifact) => sum + artifact.size_in_bytes, 0),
    artifactCount: activeArtifacts.length,
    cacheBytes: usage.active_caches_size_in_bytes,
    cacheCount: caches.length,
    duplicates: duplicateCacheGroups(caches),
    closedPr: closedPrCaches(caches, states),
  };
}

/** Escape a value for a Markdown table cell, backslashes first. */
export function markdownCell(value: string): string {
  return value.replace(/\\/g, "\\\\").replace(/\|/g, "\\|");
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

export function formatGbHours(value: number): string {
  return `${value.toFixed(1)} GB-hours`;
}

export function formatPercent(value: number): string {
  return `${value.toFixed(1)}%`;
}

export function formatUsd(value: number): string {
  return `$${value.toFixed(value > 0 && value < 1 ? 4 : 2)}`;
}

const normalizeSku = (sku: string): string =>
  sku
    .trim()
    .toLowerCase()
    .replace(/[\s-]+/g, "_");

/** Detailed usage uses `Actions storage`; the summary uses `actions_storage`. */
export function isActionsStorageSku(sku: string): boolean {
  return normalizeSku(sku) === "actions_storage";
}

export function isPackagesStorageSku(sku: string): boolean {
  const normalized = normalizeSku(sku);
  return normalized === "packages_storage" || normalized.startsWith("packages_");
}

const isGbHours = (item: { unitType: string }): boolean => /gigabyte.?hour/i.test(item.unitType);

export function sumGbHours(items: BillingUsageItem[]): number {
  return items.filter((item) => isGbHours(item)).reduce((sum, item) => sum + item.quantity, 0);
}

function isPrivateRepo(name: string | undefined, privateRepos: Set<string>): boolean {
  if (name === undefined) {
    return false;
  }
  return privateRepos.has(name) || privateRepos.has(name.split("/")[1] ?? "");
}

/** First day the cumulative billable meter crossed the monthly allowance. */
function firstDayOver(items: BillingUsageItem[], allowance: number): string | undefined {
  const byDay = new Map<string, number>();
  for (const item of items) {
    if (!isGbHours(item)) {
      continue;
    }
    const day = item.date.slice(0, 10);
    byDay.set(day, (byDay.get(day) ?? 0) + item.quantity);
  }
  let cumulative = 0;
  for (const day of [...byDay.keys()].sort()) {
    cumulative += byDay.get(day) ?? 0;
    if (cumulative > allowance) {
      return day;
    }
  }
  return undefined;
}

/**
 * Turn the billing meter into the quota facts that current bytes cannot show.
 *
 * `privateRepos` is the set of repositories whose artifact storage is billed;
 * public repositories are free and are subtracted so the usage is comparable
 * with the 500 MB allowance. Packages share the same allowance.
 */
export function quotaReport(input: {
  summary: BillingSummaryResponse;
  detailed: BillingUsageItem[];
  privateRepos: Set<string>;
  currentArtifactBytes: number;
  now: Date;
  allowanceGbHours?: number;
}): QuotaReport {
  const summaryItems = input.summary.usageItems ?? [];
  const actionsSummary = summaryItems.find((item) => isActionsStorageSku(item.sku));
  const packagesSummary = summaryItems.find((item) => isPackagesStorageSku(item.sku));
  const detailedActions = input.detailed.filter((item) => isActionsStorageSku(item.sku));
  const detailedPackages = input.detailed.filter((item) => isPackagesStorageSku(item.sku));

  if (detailedActions.length === 0 && (actionsSummary?.grossQuantity ?? 0) > 0) {
    return { available: false, error: "Detailed billing usage is unavailable for Actions storage" };
  }

  const privateActions = detailedActions.filter((item) => isPrivateRepo(item.repositoryName, input.privateRepos));
  const publicActions = detailedActions.filter((item) => !isPrivateRepo(item.repositoryName, input.privateRepos));
  const actionsGbHours = sumGbHours(privateActions);
  const publicActionsGbHours = sumGbHours(publicActions);
  const packagesGbHours = packagesSummary?.grossQuantity ?? sumGbHours(detailedPackages);
  const combinedGbHours = actionsGbHours + packagesGbHours;
  const allowanceGbHours = input.allowanceGbHours ?? ALLOWANCE_GB_HOURS;
  const usedPercent = allowanceGbHours > 0 ? (combinedGbHours / allowanceGbHours) * 100 : 0;
  const summaryNetGbHours = (actionsSummary?.netQuantity ?? 0) + (packagesSummary?.netQuantity ?? 0);
  const netGbHours =
    actionsSummary === undefined && packagesSummary === undefined
      ? Math.max(0, combinedGbHours - allowanceGbHours)
      : summaryNetGbHours;
  const pricePerGbHour = actionsSummary?.pricePerUnit ?? FALLBACK_PRICE_PER_GB_HOUR;

  const year = input.now.getUTCFullYear();
  const monthIndex = input.now.getUTCMonth();
  const daysInMonth = new Date(Date.UTC(year, monthIndex + 1, 0)).getUTCDate();
  const monthStart = new Date(Date.UTC(year, monthIndex, 1));
  const nextMonth = new Date(Date.UTC(year, monthIndex + 1, 1));
  const hoursElapsed = Math.max(0, (input.now.getTime() - monthStart.getTime()) / 3_600_000);
  const hoursInMonth = (nextMonth.getTime() - monthStart.getTime()) / 3_600_000;
  const remainingHours = Math.max(0, hoursInMonth - hoursElapsed);

  const currentArtifactGb = input.currentArtifactBytes / 1024 ** 3;
  const packageDays = detailedPackages
    .filter((item) => isGbHours(item))
    .map((item) => item.date.slice(0, 10))
    .sort();
  const latestPackageDay = packageDays[packageDays.length - 1];
  const packagesCurrentGb =
    latestPackageDay === undefined
      ? 0
      : sumGbHours(detailedPackages.filter((item) => item.date.startsWith(latestPackageDay))) / 24;
  const currentGb = currentArtifactGb + packagesCurrentGb;

  const projectedGbHours = combinedGbHours + currentGb * remainingHours;
  const projectedOverGbHours = Math.max(0, projectedGbHours - allowanceGbHours);
  const exhausted = combinedGbHours > allowanceGbHours;
  const exhaustedOn = firstDayOver([...privateActions, ...detailedPackages], allowanceGbHours);

  let projectedExhaustion: string | undefined;
  if (!exhausted) {
    if (currentGb <= 0) {
      projectedExhaustion = "not projected (no billable storage)";
    } else {
      const hoursToExhaustion = (allowanceGbHours - combinedGbHours) / currentGb;
      projectedExhaustion =
        hoursToExhaustion > remainingHours
          ? "after month end"
          : new Date(input.now.getTime() + hoursToExhaustion * 3_600_000).toISOString().slice(0, 10);
    }
  }

  const summaryCost = (actionsSummary?.netAmount ?? 0) + (packagesSummary?.netAmount ?? 0);
  const accruedCost = summaryCost > 0 ? summaryCost : netGbHours * pricePerGbHour;

  return {
    available: true,
    month: `${year}-${String(monthIndex + 1).padStart(2, "0")}`,
    day: input.now.getUTCDate(),
    daysInMonth,
    actionsGbHours,
    publicActionsGbHours,
    packagesGbHours,
    combinedGbHours,
    allowanceGbHours,
    usedPercent,
    netGbHours,
    exhausted,
    exhaustedOn,
    currentArtifactBytes: input.currentArtifactBytes,
    currentGb,
    projectedGbHours,
    projectedOverGbHours,
    accruedCost,
    projectedCost: projectedOverGbHours * pricePerGbHour,
    projectedExhaustion,
  };
}

export function renderQuotaSection(quota: QuotaReport): string[] {
  const lines: string[] = ["## Artifact quota (accrued)", ""];
  if (!quota.available) {
    lines.push(
      `Billing usage unavailable: ${markdownCell(quota.error)}`,
      "",
      "Grant the report token organization billing read (for example, ORG_BILLING_TOKEN) to include month-to-date GB-hours.",
      "",
    );
    return lines;
  }

  let status = "**Within allowance**";
  if (quota.exhausted) {
    status = `**Exhausted${quota.exhaustedOn === undefined ? "" : ` on ${quota.exhaustedOn}`}**`;
  } else if (
    quota.projectedExhaustion !== undefined &&
    quota.projectedExhaustion !== "after month end" &&
    !quota.projectedExhaustion.startsWith("not projected")
  ) {
    status = `**Within allowance**; projected exhaustion ${quota.projectedExhaustion}`;
  }

  lines.push(
    "GitHub blocks uploads on accrued GB-hours, not on stored bytes. The 500 MB allowance is shared with Packages; caches are separate.",
    "",
    "| Metric | Value |",
    "|---|---:|",
    `| Billing period | ${quota.month}, day ${quota.day} of ${quota.daysInMonth} |`,
    `| Billable Actions storage | ${formatGbHours(quota.actionsGbHours)} |`,
    `| Public repository storage (free) | ${formatGbHours(quota.publicActionsGbHours)} |`,
    `| Packages storage | ${formatGbHours(quota.packagesGbHours)} |`,
    `| Combined billable | ${formatGbHours(quota.combinedGbHours)} |`,
    `| Monthly allowance (500 MB) | ${formatGbHours(quota.allowanceGbHours)} |`,
    `| Allowance used | ${formatPercent(quota.usedPercent)} |`,
    `| Billing net overage | ${formatGbHours(quota.netGbHours)} |`,
    `| Status | ${status} |`,
    `| Current billable artifacts | ${formatBytes(quota.currentArtifactBytes)} |`,
    `| Projected month-end | ${formatGbHours(quota.projectedGbHours)} |`,
    `| Projected overage | ${formatGbHours(quota.projectedOverGbHours)} |`,
    `| Accrued cost | ${formatUsd(quota.accruedCost)} |`,
    `| Projected cost | ${formatUsd(quota.projectedCost)} |`,
    "",
    "Projection holds current billable storage flat for the rest of the month. Costs use the billing API net amount and the shared-storage rate.",
    "",
  );
  return lines;
}

export function renderReport(
  reports: RepoReport[],
  failures: QuotaFailure[],
  generatedAt: string,
  quota?: QuotaReport,
): string {
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

  if (quota !== undefined) {
    lines.push(...renderQuotaSection(quota));
  }

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
        `| ${failure.repo} | [run](${failure.run}) | ${markdownCell(failure.job)} | ${markdownCell(
          failure.step,
        )} | \`${markdownCell(failure.line)}\` |`,
      );
    }
    lines.push("");
  }

  return lines.join("\n");
}

async function privateRepoNameSet(reports: RepoReport[]): Promise<Set<string>> {
  const names = new Set<string>();
  if (ORG !== "") {
    try {
      for (let page = 1; page <= 10; page += 1) {
        const batch = await request<Array<{ name: string; full_name: string }>>(
          `/orgs/${ORG}/repos?type=private&per_page=100&page=${page}`,
        );
        for (const repo of batch) {
          names.add(repo.name);
          const short = repo.full_name.split("/")[1];
          if (short !== undefined) {
            names.add(short);
          }
        }
        if (batch.length < 100) {
          break;
        }
      }
    } catch {
      // Fall back to the configured repositories below. An unlisted private
      // repository is still visible in the billing detail but is counted as
      // public until the token can list organisation repositories.
    }
  }
  for (const report of reports) {
    if (report.private) {
      const short = report.repo.split("/")[1];
      if (short !== undefined) {
        names.add(short);
      }
    }
  }
  return names;
}

async function buildQuotaReport(reports: RepoReport[], now: Date): Promise<QuotaReport> {
  if (ORG === "") {
    return { available: false, error: "STORAGE_ORG is empty" };
  }
  try {
    const query = `year=${now.getUTCFullYear()}&month=${now.getUTCMonth() + 1}`;
    const [summary, detailed] = await Promise.all([
      request<BillingSummaryResponse>(`/orgs/${ORG}/settings/billing/usage/summary?${query}`, BILLING_TOKEN),
      request<BillingUsageResponse>(`/orgs/${ORG}/settings/billing/usage?${query}`, BILLING_TOKEN),
    ]);
    const privateRepos = await privateRepoNameSet(reports);
    const currentArtifactBytes = reports
      .filter((report) => report.private)
      .reduce((sum, report) => sum + report.artifactBytes, 0);
    return quotaReport({
      summary,
      detailed: detailed.usageItems ?? [],
      privateRepos,
      currentArtifactBytes,
      now,
    });
  } catch (error) {
    return { available: false, error: error instanceof Error ? error.message : String(error) };
  }
}

if (import.meta.main) {
  if (TOKEN === "") {
    console.error("❌ GITHUB_TOKEN (or GH_TOKEN) is required to read Actions storage");
    process.exit(2);
  }

  const now = new Date();
  const generatedAt = now.toISOString();
  const since = new Date(now.getTime() - REPORT_DAYS * 86_400_000).toISOString();

  const reports: RepoReport[] = [];
  for (const repo of REPOS) {
    reports.push(await repoReport(repo));
  }

  const quota = await buildQuotaReport(reports, now);
  if (!quota.available) {
    console.log(`::warning title=Storage report::Billing usage unavailable: ${quota.error}`);
  } else if (quota.exhausted) {
    console.log(
      `::warning title=Artifact quota::${quota.combinedGbHours.toFixed(1)} of ${quota.allowanceGbHours.toFixed(
        1,
      )} GB-hours used${quota.exhaustedOn === undefined ? "" : `; exceeded on ${quota.exhaustedOn}`}`,
    );
  }

  const failures: QuotaFailure[] = [];
  for (const repo of REPOS) {
    failures.push(...(await quotaFailures(repo, since)));
  }

  const markdown = renderReport(reports, failures, generatedAt, quota);
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
