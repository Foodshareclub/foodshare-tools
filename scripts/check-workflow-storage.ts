#!/usr/bin/env bun
// Guard the workflow and composite-action shapes that grow repository
// storage without bound.
//
// GitHub bills artifacts and Actions caches that outlive the run that made
// them. Two shapes caused a 45 GB overage, and both look green from the
// outside:
//
//   1. `actions/upload-artifact` without a tight `retention-days`. The
//      repository default can be as high as 90 days, so every push on every
//      branch leaves a copy behind for a quarter of a year.
//   2. A cache `key` containing a per-run value such as `github.sha` or
//      `github.run_id`. That key is unique every time, so every run saves a
//      brand new cache and no run ever restores one.
//
// Retention is tiered rather than one flat number:
//
//   - default: 7 days;
//   - run-scoped handoff: 1 day, when the same document downloads the
//     artifact by name or pattern later in the run;
//   - release artifacts: the explicit allowlist below may keep the default
//     cap even when a publish job downloads them.
//
// Composite actions under `.github/actions` are checked with the same rules
// as workflows. A passing build says nothing about any of this; the guard
// fails the build instead, so the pattern cannot be merged back in.
//
// Deliberately NOT checked: `if: always()` on an upload. Uploading a test
// report after a failing test is the reason that step exists; the retention
// rule already bounds what it costs.
//
// Usage: bun scripts/check-workflow-storage.ts <.github/workflows> [<.github/actions>] [<file> …]

import { stat } from "bun:fs/promises";

/** The cap for an ordinary artifact that no later job downloads. */
const DEFAULT_MAX_RETENTION_DAYS = 7;

/** An artifact downloaded later in the same run is a temporary handoff. */
const HANDOFF_MAX_RETENTION_DAYS = 1;

/**
 * Release deliverables. These are the only artifacts allowed the default cap
 * when a later job downloads them. Keep the entry explicit: a name pattern
 * here is a statement that the artifact is published, not merely convenient.
 */
const RELEASE_ARTIFACTS: ReadonlyArray<{ pattern: RegExp; maxRetentionDays: number; reason: string }> = [
  {
    pattern: /^android-aab$/,
    maxRetentionDays: 7,
    reason: "Play Store release bundle",
  },
  {
    pattern: /^foodshare-tools-/,
    maxRetentionDays: 7,
    reason: "release archives published by the release job",
  },
];

/** Values that are unique per commit, run or attempt, and so never restored. */
const PER_RUN_CACHE_VALUES: Record<string, string> = {
  "github.sha": "unique per commit",
  "github.run_id": "unique per run",
  "github.run_number": "unique per run",
  "github.run_attempt": "unique per attempt",
};

type Mapping = Record<string, unknown>;

const isMapping = (value: unknown): value is Mapping =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const asText = (value: unknown): string | undefined => (typeof value === "string" ? value : undefined);

type Violation = {
  file: string;
  step: string;
  message: string;
};

type StepGroup = {
  label: string;
  steps: unknown[];
};

type DownloadSpec = {
  name?: string;
  pattern?: string;
  all: boolean;
};

type RetentionPolicy = {
  maxDays: number;
  label: string;
};

function stepLabel(jobId: string, index: number, step: Mapping): string {
  const name = asText(step.name)?.trim();
  const uses = asText(step.uses)?.trim();
  return `${jobId} › ${name || uses || `step ${index + 1}`}`;
}

/** Workflow jobs and a composite action's `runs.steps` are the same shape. */
function stepGroups(document: Mapping): StepGroup[] {
  const groups: StepGroup[] = [];
  const jobs = isMapping(document.jobs) ? document.jobs : {};
  for (const [jobId, job] of Object.entries(jobs)) {
    if (isMapping(job) && Array.isArray(job.steps)) {
      groups.push({ label: jobId, steps: job.steps });
    }
  }

  const runs = isMapping(document.runs) ? document.runs : undefined;
  if (runs !== undefined && asText(runs.using) === "composite" && Array.isArray(runs.steps)) {
    groups.push({ label: "composite", steps: runs.steps });
  }
  return groups;
}

function downloadSpecs(groups: StepGroup[]): DownloadSpec[] {
  const specs: DownloadSpec[] = [];
  for (const group of groups) {
    for (const step of group.steps) {
      if (!isMapping(step) || !(asText(step.uses) ?? "").trim().startsWith("actions/download-artifact")) {
        continue;
      }
      const settings = isMapping(step.with) ? step.with : {};
      const name = asText(settings.name)?.trim() || undefined;
      const pattern = asText(settings.pattern)?.trim() || undefined;
      specs.push({ name, pattern, all: name === undefined && pattern === undefined });
    }
  }
  return specs;
}

/** Normalize an Actions expression so a matrix artifact can match a pattern. */
function normalizeArtifactName(name: string): string {
  return name.replace(/\$\{\{[^}]*\}\}/g, "*");
}

/** GitHub artifact patterns use `*` and `?`, not regular expressions. */
function globRegExp(pattern: string): RegExp {
  const escaped = pattern
    .replace(/[.+^${}()|[\]\\]/g, "\\$&")
    .replace(/\*/g, ".*")
    .replace(/\?/g, ".");
  return new RegExp(`^${escaped}$`);
}

function isRunScoped(name: string, downloads: DownloadSpec[]): boolean {
  const normalized = normalizeArtifactName(name);
  for (const download of downloads) {
    if (download.all) {
      return true;
    }
    if (download.name !== undefined && globRegExp(normalizeArtifactName(download.name)).test(normalized)) {
      return true;
    }
    if (download.pattern !== undefined && globRegExp(download.pattern).test(normalized)) {
      return true;
    }
  }
  return false;
}

function retentionPolicy(name: string, downloads: DownloadSpec[]): RetentionPolicy {
  const release = RELEASE_ARTIFACTS.find((entry) => entry.pattern.test(name));
  if (release !== undefined) {
    return { maxDays: release.maxRetentionDays, label: `release artifact (${release.reason})` };
  }
  if (isRunScoped(name, downloads)) {
    return { maxDays: HANDOFF_MAX_RETENTION_DAYS, label: "run-scoped handoff" };
  }
  return { maxDays: DEFAULT_MAX_RETENTION_DAYS, label: "default cap" };
}

function checkStep(file: string, jobId: string, index: number, step: Mapping, downloads: DownloadSpec[]): Violation[] {
  const uses = (asText(step.uses) ?? "").trim();
  const settings = isMapping(step.with) ? step.with : {};
  const stepName = stepLabel(jobId, index, step);
  const found: Violation[] = [];

  // upload-artifact and its `merge` sibling both accept retention-days.
  if (uses.startsWith("actions/upload-artifact")) {
    const declared = settings["retention-days"];
    const days =
      typeof declared === "number"
        ? declared
        : typeof declared === "string" && declared.trim() !== ""
          ? Number(declared)
          : Number.NaN;
    const artifactName = asText(settings.name)?.trim() ?? "";
    const policy = retentionPolicy(artifactName, downloads);

    const declaredPresent = declared !== undefined && declared !== null && String(declared).trim() !== "";

    if (!declaredPresent) {
      found.push({
        file,
        step: stepName,
        message: `upload-artifact sets no retention-days; ${policy.label} allows at most ${policy.maxDays} day(s) — add an explicit cap`,
      });
    } else if (!Number.isInteger(days)) {
      // A value like ${{ inputs.days }} cannot be checked here, and a cap that
      // cannot be checked is not a cap. Say which case this is, so the fix is
      // obvious from the failure rather than from reading this script.
      found.push({
        file,
        step: stepName,
        message: `retention-days is ${JSON.stringify(declared)}, which is not a literal day count — write the number`,
      });
    } else if (days < 1) {
      found.push({
        file,
        step: stepName,
        message: `retention-days is ${days}; keep it at least 1 day`,
      });
    } else if (days > policy.maxDays) {
      found.push({
        file,
        step: stepName,
        message: `retention-days is ${days}; ${policy.label} allows at most ${policy.maxDays} day(s)`,
      });
    }
  }

  // cache, cache/restore and cache/save all key on `with.key`. Each of these
  // values changes on its own schedule, so a key containing one is created
  // fresh every time it changes and restored by nobody.
  if (uses.startsWith("actions/cache")) {
    const key = asText(settings.key) ?? "";
    for (const [token, nature] of Object.entries(PER_RUN_CACHE_VALUES)) {
      if (key.includes(token)) {
        found.push({
          file,
          step: stepName,
          message: `cache key contains ${token}, which is ${nature} — every run saves a cache that nothing restores`,
        });
        break;
      }
    }
  }

  return found;
}

function checkDocument(file: string, document: unknown): Violation[] {
  if (!isMapping(document)) {
    return [];
  }

  const groups = stepGroups(document);
  const downloads = downloadSpecs(groups);
  const found: Violation[] = [];

  for (const group of groups) {
    group.steps.forEach((step, index) => {
      if (isMapping(step)) {
        found.push(...checkStep(file, group.label, index, step, downloads));
      }
    });
  }

  return found;
}

/**
 * Expand a directory to the YAML files that can hold steps: workflows directly
 * inside it, and composite actions anywhere below it. Missing directories are
 * ignored so callers can pass `.github/actions` unconditionally.
 */
async function expand(target: string): Promise<string[]> {
  let info: Awaited<ReturnType<typeof stat>>;
  try {
    info = await stat(target);
  } catch {
    return [];
  }

  if (info.isFile()) {
    return [target];
  }
  if (!info.isDirectory()) {
    return [];
  }

  const prefix = target.replace(/\/+$/, "");
  const files = new Set<string>();
  const topLevel = new Bun.Glob("*.{yml,yaml}");
  for await (const match of topLevel.scan({ cwd: target, onlyFiles: true })) {
    files.add(`${prefix}/${match}`);
  }
  const composite = new Bun.Glob("**/action.{yml,yaml}");
  for await (const match of composite.scan({ cwd: target, onlyFiles: true, dot: true })) {
    const parts = match.split("/");
    if (!parts.includes("node_modules") && !parts.includes(".git")) {
      files.add(`${prefix}/${match}`);
    }
  }
  return [...files].sort();
}

const targets = Bun.argv.slice(2);

if (targets.length === 0) {
  console.log("No workflow files to check");
  process.exit(0);
}

const files = new Set<string>();
for (const target of targets) {
  for (const file of await expand(target)) {
    files.add(file);
  }
}

if (files.size === 0) {
  console.error(`❌ ${targets.join(", ")}: no workflow or composite-action files found`);
  process.exit(1);
}

const violations: Violation[] = [];

for (const file of files) {
  let document: unknown;
  try {
    document = Bun.YAML.parse(await Bun.file(file).text());
  } catch (error) {
    violations.push({
      file,
      step: "parse",
      message: `not valid YAML: ${error instanceof Error ? error.message : String(error)}`,
    });
    continue;
  }
  violations.push(...checkDocument(file, document));
}

if (violations.length > 0) {
  for (const violation of violations) {
    console.error(`❌ ${violation.file} › ${violation.step}: ${violation.message}`);
  }
  console.error(`\n${violations.length} storage problem(s) across ${files.size} file(s)`);
  process.exit(1);
}

console.log(`✅ ${files.size} workflow/composite-action file(s) keep artifacts and caches bounded`);
