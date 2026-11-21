#!/usr/bin/env bun
// Audit workflow storage across every repository, independently of each
// repository's own CI.
//
// The per-repository storage-hygiene job only runs when that repository's CI
// runs. A queued or disabled pipeline can therefore leave a bad retention or
// cache key sitting on the default branch while its guard never sees it. This
// audit fetches the default-branch YAML directly through the API and applies
// the same guard: workflows under `.github/workflows`, and composite actions
// under `.github/actions`.
//
// Usage: GITHUB_TOKEN=... bun scripts/audit-workflow-storage.ts
// Env:   STORAGE_REPOS       comma-separated owner/name list
//        STORAGE_AUDIT_REF   optional branch, tag or SHA for every repository
//                            (default: each repository's default branch)
//        GITHUB_API_URL      API base (default https://api.github.com)
//        GITHUB_TOKEN        token with contents read on every repository
//                            (GH_TOKEN is accepted as a fallback)

import { mkdir, mkdtemp, rm, writeFile } from "bun:fs/promises";
import { tmpdir } from "bun:os";
import { dirname, join } from "bun:path";

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
const REF = (process.env.STORAGE_AUDIT_REF ?? "").trim();
const CHECKER = new URL("./check-workflow-storage.ts", import.meta.url).pathname;

type TreeEntry = { path: string; type: string };
type TreeResponse = { tree?: TreeEntry[]; truncated?: boolean };
type CommitResponse = { commit?: { tree?: { sha?: string } } };
type RepoResponse = { default_branch?: string };

function headers(accept = "application/vnd.github+json"): Record<string, string> {
  return {
    authorization: `Bearer ${TOKEN}`,
    accept,
    "user-agent": "foodshare-storage-audit",
    "x-github-api-version": "2022-11-28",
  };
}

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`${API}${path}`, { headers: headers() });
  if (!response.ok) {
    const body = (await response.text()).slice(0, 300);
    throw new Error(`GET ${path} -> ${response.status}: ${body}`);
  }
  return (await response.json()) as T;
}

async function getRaw(repo: string, ref: string, path: string): Promise<string> {
  const encoded = path.split("/").map(encodeURIComponent).join("/");
  const response = await fetch(`${API}/repos/${repo}/contents/${encoded}?ref=${encodeURIComponent(ref)}`, {
    headers: headers("application/vnd.github.raw"),
  });
  if (!response.ok) {
    const body = (await response.text()).slice(0, 300);
    throw new Error(`GET ${path} in ${repo}@${ref} -> ${response.status}: ${body}`);
  }
  return response.text();
}

/** The YAML files the per-repository guard checks. */
export function auditablePath(path: string): boolean {
  return (
    /^\.github\/workflows\/[^/]+\.(?:yml|yaml)$/.test(path) ||
    /^\.github\/actions\/(?:.+\/)?action\.(?:yml|yaml)$/.test(path)
  );
}

function assertRepo(repo: string): void {
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repo)) {
    throw new Error(`invalid repository name: ${repo}`);
  }
}

async function refFor(repo: string): Promise<string> {
  if (REF !== "") {
    return REF;
  }
  const meta = await getJson<RepoResponse>(`/repos/${repo}`);
  if (meta.default_branch === undefined || meta.default_branch === "") {
    throw new Error(`could not read the default branch for ${repo}`);
  }
  return meta.default_branch;
}

async function yamlPaths(repo: string, ref: string): Promise<string[]> {
  const commit = await getJson<CommitResponse>(`/repos/${repo}/commits/${encodeURIComponent(ref)}`);
  const treeSha = commit.commit?.tree?.sha;
  if (treeSha === undefined) {
    throw new Error(`could not resolve ${repo}@${ref} to a tree`);
  }
  const tree = await getJson<TreeResponse>(`/repos/${repo}/git/trees/${treeSha}?recursive=1`);
  if (tree.truncated === true) {
    throw new Error(`${repo}@${ref}: git tree is truncated; refusing a partial audit`);
  }
  return (tree.tree ?? [])
    .filter((entry) => entry.type === "blob" && auditablePath(entry.path))
    .map((entry) => entry.path)
    .sort();
}

async function auditRepo(
  repo: string,
  root: string,
): Promise<{ files: number; ref: string; output: string; failed: boolean }> {
  assertRepo(repo);
  const ref = await refFor(repo);
  const paths = await yamlPaths(repo, ref);
  if (paths.length === 0) {
    throw new Error(`${repo}@${ref}: no workflow or composite-action YAML found`);
  }

  // The repository name is validated as owner/name; flattening the separator
  // keeps it from introducing an extra path segment.
  const repoRoot = join(root, repo.replace("/", "__"));
  for (const path of paths) {
    const content = await getRaw(repo, ref, path);
    const full = join(repoRoot, path);
    await mkdir(dirname(full), { recursive: true });
    await writeFile(full, content);
  }

  const result = Bun.spawnSync(
    [process.execPath, CHECKER, join(repoRoot, ".github/workflows"), join(repoRoot, ".github/actions")],
    { stdout: "pipe", stderr: "pipe" },
  );
  const stdout = new TextDecoder().decode(result.stdout);
  const stderr = new TextDecoder().decode(result.stderr);
  const output = `${stdout}${stderr}`.replaceAll(`${repoRoot}/`, `${repo}/`).trim();

  return {
    files: paths.length,
    ref,
    output,
    failed: result.exitCode !== 0,
  };
}

async function main(): Promise<number> {
  if (TOKEN === "") {
    console.error("❌ GITHUB_TOKEN (or GH_TOKEN) is required to audit the repositories");
    return 2;
  }
  if (REPOS.length === 0) {
    console.error("❌ STORAGE_REPOS is empty");
    return 2;
  }

  const lines: string[] = ["# Central workflow storage audit", "", `Generated ${new Date().toISOString()}.`, ""];
  let failures = 0;
  const root = await mkdtemp(join(tmpdir(), "storage-audit-"));

  try {
    for (const repo of REPOS) {
      console.log(`Auditing ${repo}...`);
      try {
        const result = await auditRepo(repo, root);
        lines.push(`## ${repo}`, "", `Ref: \`${result.ref}\``, "", result.output, "");
        if (result.failed) {
          failures += 1;
          console.error(`::error title=Storage audit::${repo} failed the storage guard`);
        } else {
          console.log(`✅ ${repo}: ${result.files} file(s) at ${result.ref}`);
        }
      } catch (error) {
        failures += 1;
        const message = error instanceof Error ? error.message : String(error);
        lines.push(`## ${repo}`, "", `❌ ${message}`, "");
        console.error(`::error title=Storage audit::${repo}: ${message}`);
      }
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }

  lines.push(
    "",
    failures === 0
      ? `✅ ${REPOS.length} repositories pass the storage guard.`
      : `❌ ${failures} of ${REPOS.length} repositories failed the audit.`,
  );
  const markdown = lines.join("\n");
  const summary = process.env.GITHUB_STEP_SUMMARY;
  if (summary) {
    await Bun.write(summary, `${markdown}\n`);
  } else {
    console.log(`\n${markdown}`);
  }

  return failures === 0 ? 0 : 1;
}

if (import.meta.main) {
  process.exitCode = await main();
}
