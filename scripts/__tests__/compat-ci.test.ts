import { expect, test } from "bun:test";
import { checkRepositoryCI, inspectionExitCode } from "../compat-ci";

const sha = "a".repeat(40);
const oldSha = "b".repeat(40);
const repo = "Foodshareclub/foodshare-backend";
const completed = { head_sha: sha, head_branch: "main", event: "push", status: "completed", conclusion: "success" };

function fixture(runs: unknown[], changed = false) {
  const calls: string[] = [];
  let heads = 0;
  return {
    calls,
    async read(path: string): Promise<unknown> {
      calls.push(path);
      if (path.endsWith("/commits/main")) return { sha: changed && heads++ > 0 ? oldSha : sha };
      if (path.includes("/actions/workflows/")) return { workflow_runs: runs };
      // This newer successful probe is deliberately not the required pipeline.
      if (path.includes("/actions/runs")) return { workflow_runs: [completed] };
      throw new Error("Unexpected API path");
    },
  };
}

test("a successful probe cannot mask a failed backend pipeline", async () => {
  const api = fixture([{ ...completed, conclusion: "failure" }]);
  expect((await checkRepositoryCI(repo, api.read)).status).toBe("FAIL");
  expect(api.calls.some((path) => path.includes("/actions/workflows/backend.yml/runs?"))).toBe(true);
  expect(api.calls.some((path) => path.includes("/actions/runs?"))).toBe(false);
  expect(api.calls.some((path) => path.includes(`head_sha=${sha}`))).toBe(true);
});

test("success is scoped to the exact current main commit", async () => {
  expect((await checkRepositoryCI(repo, fixture([completed]).read)).status).toBe("PASS");
  expect((await checkRepositoryCI(repo, fixture([{ ...completed, head_sha: oldSha }]).read)).status).toBe("WARN");
  expect((await checkRepositoryCI(repo, fixture([completed], true).read)).status).toBe("WARN");
});

test("queued, cancelled and skipped checks never establish passing validation", async () => {
  for (const status of ["queued", "in_progress", "waiting", "pending", "requested"]) {
    expect((await checkRepositoryCI(repo, fixture([{ ...completed, status, conclusion: null }]).read)).status).toBe(
      "WARN",
    );
  }
  for (const conclusion of ["cancelled", "skipped", "neutral"]) {
    expect((await checkRepositoryCI(repo, fixture([{ ...completed, conclusion }]).read)).status).toBe("WARN");
  }
});

test("only an explicit exception demotes a real pipeline failure", async () => {
  const api = fixture([{ ...completed, conclusion: "timed_out" }]);
  expect((await checkRepositoryCI(repo, api.read)).status).toBe("FAIL");
  const ignored = await checkRepositoryCI(repo, api.read, true);
  expect(ignored.status).toBe("WARN");
  expect(ignored.detail).toContain("explicit exception");
  expect(inspectionExitCode([ignored])).toBe(0);
});

test("incomplete verification is distinct from a passing inspection", async () => {
  expect(inspectionExitCode([await checkRepositoryCI(repo, fixture([completed]).read)])).toBe(0);
  expect(
    inspectionExitCode([
      await checkRepositoryCI(repo, fixture([{ ...completed, status: "queued", conclusion: null }]).read),
    ]),
  ).toBe(2);
  expect(
    inspectionExitCode([await checkRepositoryCI(repo, fixture([{ ...completed, conclusion: "failure" }]).read)]),
  ).toBe(1);
  expect(inspectionExitCode([])).toBe(1);
});

test("inaccessible or malformed inspection fails without echoing API errors", async () => {
  const unavailable = await checkRepositoryCI(
    repo,
    async () => {
      throw new Error("private diagnostic");
    },
    true,
  );
  expect(unavailable.status).toBe("FAIL");
  expect(unavailable.detail).not.toContain("private diagnostic");
  expect((await checkRepositoryCI(repo, async () => ({ sha: "invalid" }))).status).toBe("FAIL");
  expect((await checkRepositoryCI(repo, fixture([{ ...completed, conclusion: null }]).read)).status).toBe("FAIL");
});

test("each repository checks its declared primary workflow", async () => {
  for (const [name, workflow] of Object.entries({
    "foodshare-backend": "backend.yml",
    "foodshare-web": "web.yml",
    "foodshare-app": "android.yml",
    "foodshare-tools": "ci.yml",
  })) {
    const api = fixture([completed]);
    expect((await checkRepositoryCI(`Foodshareclub/${name}`, api.read)).status).toBe("PASS");
    expect(api.calls.some((path) => path.includes(`/actions/workflows/${workflow}/runs?`))).toBe(true);
  }
});

test("an empty repository list cannot produce an empty successful inspection", async () => {
  const command = Bun.spawn([process.execPath, new URL("../compat-ci.ts", import.meta.url).pathname], {
    env: { ...Bun.env, CHECK_REPOS: "" },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [code, output] = await Promise.all([command.exited, new Response(command.stdout).text()]);
  expect(code).toBe(1);
  expect(output).toContain("No repositories configured");
});
