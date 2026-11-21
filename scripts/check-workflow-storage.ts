#!/usr/bin/env bun
// Guard the workflow shapes that grow repository storage without bound.
//
// GitHub bills artifacts and Actions caches that outlive the run that made
// them. Two shapes caused a 45 GB overage, and both look green from the
// outside:
//
//   1. `actions/upload-artifact` without `retention-days`. The fallback is the
//      repository default of 90 days, so every push on every branch leaves a
//      copy behind for a quarter of a year.
//   2. A cache `key` containing a per-run value such as `github.sha` or
//      `github.run_id`. That key is unique every time, so every run saves a
//      brand new cache and no run ever restores one.
//
// A passing build says nothing about either. This check fails the build
// instead, so the pattern cannot be merged back in.
//
// Deliberately NOT checked: `if: always()` on an upload. Uploading a test
// report after a failing test is the reason that step exists; the retention
// rule already bounds what it costs.
//
// Usage: bun scripts/check-workflow-storage.ts <.github/workflows> [<file> …]

/** The longest retention a workflow may ask for. The default is 90. */
const MAX_RETENTION_DAYS = 90;

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

function stepLabel(jobId: string, index: number, step: Mapping): string {
  const name = asText(step.name)?.trim();
  const uses = asText(step.uses)?.trim();
  return `${jobId} › ${name || uses || `step ${index + 1}`}`;
}

function checkStep(file: string, jobId: string, index: number, step: Mapping): Violation[] {
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

    const declaredPresent = declared !== undefined && declared !== null && String(declared).trim() !== "";

    if (!declaredPresent) {
      found.push({
        file,
        step: stepName,
        message:
          "upload-artifact sets no retention-days, so the 90-day repository default applies — add an explicit cap",
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
    } else if (days < 1 || days > MAX_RETENTION_DAYS) {
      found.push({
        file,
        step: stepName,
        message: `retention-days is ${days}; keep it between 1 and ${MAX_RETENTION_DAYS}`,
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

  const jobs = isMapping(document.jobs) ? document.jobs : {};
  const found: Violation[] = [];

  for (const [jobId, job] of Object.entries(jobs)) {
    if (!isMapping(job) || !Array.isArray(job.steps)) {
      continue;
    }
    job.steps.forEach((step, index) => {
      if (isMapping(step)) {
        found.push(...checkStep(file, jobId, index, step));
      }
    });
  }

  return found;
}

/** Expand a directory argument to the workflows directly inside it. */
async function expand(target: string): Promise<string[]> {
  const stat = await Bun.file(target).exists();
  if (stat) {
    return [target];
  }

  const glob = new Bun.Glob("*.{yml,yaml}");
  const files: string[] = [];
  for await (const match of glob.scan({ cwd: target, onlyFiles: true })) {
    files.push(`${target.replace(/\/+$/, "")}/${match}`);
  }
  return files.sort();
}

const targets = Bun.argv.slice(2);

if (targets.length === 0) {
  console.log("No workflow files to check");
  process.exit(0);
}

const files: string[] = [];
for (const target of targets) {
  const expanded = await expand(target);
  if (expanded.length === 0) {
    console.error(`❌ ${target}: no .yml or .yaml files found`);
    process.exit(1);
  }
  files.push(...expanded);
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
  console.error(`\n${violations.length} storage problem(s) across ${files.length} workflow file(s)`);
  process.exit(1);
}

console.log(`✅ ${files.length} workflow file(s) keep artifacts and caches bounded`);
