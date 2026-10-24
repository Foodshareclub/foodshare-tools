type ObjectValue = Record<string, unknown>;
export type CIResult = { repo: string; status: "PASS" | "WARN" | "FAIL"; detail: string; explicitException?: boolean };
type ReadAPI = (path: string) => Promise<unknown>;

const workflows: Record<string, string> = {
  "foodshare-backend": "backend.yml",
  "foodshare-web": "web.yml",
  "foodshare-app": "android.yml",
  "foodshare-tools": "ci.yml",
};

function object(value: unknown): ObjectValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Invalid GitHub response");
  return value as ObjectValue;
}

async function mainCommit(repo: string, read: ReadAPI): Promise<string> {
  const sha = object(await read(`repos/${repo}/commits/main`)).sha;
  if (typeof sha !== "string" || !/^[a-f0-9]{40}$/i.test(sha)) throw new Error("Invalid main commit");
  return sha;
}

/** A successful probe or old commit must not mask the required pipeline. */
export async function checkRepositoryCI(repo: string, read: ReadAPI, ignoreFailure = false): Promise<CIResult> {
  const result = (status: CIResult["status"], detail: string): CIResult => ({ repo, status, detail });
  if (!/^[a-z0-9_.-]+\/[a-z0-9_.-]+$/i.test(repo)) {
    return { repo: "invalid repository", status: "FAIL", detail: "Invalid repository name" };
  }
  const workflow = workflows[repo.split("/")[1].toLowerCase()];
  if (typeof workflow !== "string") return result("FAIL", "No required workflow configured for this repository");
  try {
    const sha = await mainCommit(repo, read);
    const response = object(
      await read(`repos/${repo}/actions/workflows/${workflow}/runs?branch=main&event=push&head_sha=${sha}&per_page=20`),
    );
    if (!Array.isArray(response.workflow_runs)) throw new Error("Invalid workflow response");
    const runs = response.workflow_runs.map(object);
    const run = runs.find((item) => item.head_sha === sha && item.head_branch === "main" && item.event === "push");
    if ((await mainCommit(repo, read)) !== sha) return result("WARN", "main changed during inspection; rerun required");
    const label = `${workflow} at ${sha.slice(0, 7)}`;
    if (!run) return result("WARN", `${label}: no push run for this commit`);
    if (run.status !== "completed") {
      if (!["queued", "in_progress", "waiting", "pending", "requested"].includes(String(run.status))) {
        throw new Error("Invalid workflow state");
      }
      return result("WARN", `${label}: ${run.status}; validation pending`);
    }
    if (run.conclusion === "success") return result("PASS", `${label}: success`);
    if (["cancelled", "skipped", "neutral"].includes(String(run.conclusion))) {
      return result("WARN", `${label}: ${run.conclusion}; validation not established`);
    }
    if (!["failure", "timed_out", "action_required", "stale", "startup_failure"].includes(String(run.conclusion))) {
      throw new Error("Invalid workflow conclusion");
    }
    return {
      ...result(
        ignoreFailure ? "WARN" : "FAIL",
        `${label}: ${run.conclusion}${ignoreFailure ? " (explicit exception)" : ""}`,
      ),
      explicitException: ignoreFailure,
    };
  } catch {
    // Do not echo API payloads, transport URLs or credential-bearing errors.
    return result("FAIL", "Could not verify the required pipeline for main");
  }
}

export function inspectionExitCode(rows: CIResult[]): number {
  if (rows.length === 0 || rows.some((row) => row.status === "FAIL")) return 1;
  if (rows.some((row) => row.status !== "PASS" && !row.explicitException)) return 2;
  return 0;
}

async function token(): Promise<string | undefined> {
  if (Bun.env.GH_TOKEN) return Bun.env.GH_TOKEN;
  if (Bun.env.GITHUB_TOKEN) return Bun.env.GITHUB_TOKEN;
  try {
    const command = Bun.spawn(["gh", "auth", "token"], { stdout: "pipe", stderr: "ignore" });
    const [value, code] = await Promise.all([new Response(command.stdout).text(), command.exited]);
    return code === 0 ? value.trim() || undefined : undefined;
  } catch {
    return undefined;
  }
}

if (import.meta.main) {
  const repos = [
    ...new Set(
      (
        Bun.env.CHECK_REPOS ??
        "Foodshareclub/foodshare-backend Foodshareclub/foodshare-web Foodshareclub/foodshare-app Foodshareclub/foodshare-tools"
      )
        .split(/\s+/)
        .filter(Boolean),
    ),
  ];
  if (repos.length === 0) {
    console.log("| CI inspection | FAIL | No repositories configured | ");
    process.exit(1);
  }

  const credential = await token();
  const read: ReadAPI = async (path) => {
    const response = await fetch(`https://api.github.com/${path}`, {
      headers: {
        Accept: "application/vnd.github+json",
        "User-Agent": "FoodShare-Compatibility",
        "X-GitHub-Api-Version": "2022-11-28",
        ...(credential ? { Authorization: `Bearer ${credential}` } : {}),
      },
      signal: AbortSignal.timeout(15_000),
    });
    if (!response.ok) throw new Error("GitHub request failed");
    return await response.json();
  };
  const ignored = new Set((Bun.env.IGNORE_CI_FAIL ?? "").split(/\s+/));
  const rows = await Promise.all(repos.map((repo) => checkRepositoryCI(repo, read, ignored.has(repo))));
  for (const row of rows) console.log(`| CI: \`${row.repo}\` | ${row.status} | ${row.detail} |`);
  process.exitCode = inspectionExitCode(rows);
}
