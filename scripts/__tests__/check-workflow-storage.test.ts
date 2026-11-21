import { expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "bun:fs/promises";
import { tmpdir } from "bun:os";
import { dirname, join } from "bun:path";

// The guard is the only thing that stops the storage shapes from coming back.
// A regression here would be invisible: CI would stay green while the rule
// stopped firing. These tests drive the real entry point, not a copy of its
// logic.
const checker = new URL("../check-workflow-storage.ts", import.meta.url).pathname;

const HEADER = `name: sample
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
`;

function handoffWorkflow(days: number): string {
  return `name: sample
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/upload-artifact@v4
        with:
          name: build
          path: out/
          retention-days: ${days}
  test:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: build
`;
}

function releaseWorkflow(name: string, days: number, download: string): string {
  return `name: sample
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/upload-artifact@v4
        with:
          name: ${name}
          path: out/
          retention-days: ${days}
  publish:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
${download}
`;
}

async function runTree(
  files: Record<string, string>,
  targets: (root: string) => string[] = (root) => [root],
): Promise<{ code: number | null; err: string }> {
  const root = await mkdtemp(join(tmpdir(), "storage-guard-"));
  try {
    for (const [path, content] of Object.entries(files)) {
      const full = join(root, path);
      await mkdir(dirname(full), { recursive: true });
      await writeFile(full, content);
    }
    const result = Bun.spawnSync([process.execPath, checker, ...targets(root)], {
      stdout: "pipe",
      stderr: "pipe",
    });
    return { code: result.exitCode, err: result.stderr.toString() };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

async function run(workflow: string): Promise<{ code: number | null; err: string }> {
  return runTree({ "ci.yml": workflow });
}

const CAPPED = `${HEADER}      - uses: actions/upload-artifact@v4
        with:
          name: report
          path: out/
          retention-days: 7
      - uses: actions/cache@v4
        with:
          path: ~/.npm
          key: npm-\${{ hashFiles('bun.lock') }}
`;

test("a 7-day upload and a content-addressed cache key pass", async () => {
  const { code, err } = await run(CAPPED);
  expect(err).toBe("");
  expect(code).toBe(0);
});

test("an upload with no retention cap fails", async () => {
  const { code, err } = await run(`${HEADER}      - uses: actions/upload-artifact@v4
        with:
          name: report
          path: out/
`);
  expect(code).toBe(1);
  expect(err).toContain("default cap allows at most 7");
});

test("an upload above the 7-day default fails", async () => {
  const { code, err } = await run(`${HEADER}      - uses: actions/upload-artifact@v4
        with:
          name: report
          path: out/
          retention-days: 14
`);
  expect(code).toBe(1);
  expect(err).toContain("default cap allows at most 7");
});

test("a one-day artifact downloaded later in the run passes", async () => {
  const { code, err } = await run(handoffWorkflow(1));
  expect(err).toBe("");
  expect(code).toBe(0);
});

test("a handoff kept longer than one day fails", async () => {
  const { code, err } = await run(handoffWorkflow(3));
  expect(code).toBe(1);
  expect(err).toContain("run-scoped handoff allows at most 1");
});

test("a release artifact may keep the default cap when downloaded", async () => {
  const { code, err } = await run(releaseWorkflow("android-aab", 7, "          name: android-aab"));
  expect(err).toBe("");
  expect(code).toBe(0);
});

test("a release artifact above its explicit cap fails", async () => {
  const { code, err } = await run(releaseWorkflow("android-aab", 8, "          name: android-aab"));
  expect(code).toBe(1);
  expect(err).toContain("release artifact (Play Store release bundle) allows at most 7");
});

test("the release allowlist covers matrix release archives downloaded together", async () => {
  const { code, err } = await run(
    releaseWorkflow("foodshare-tools-\${{ matrix.artifact }}", 3, "          path: artifacts"),
  );
  expect(err).toBe("");
  expect(code).toBe(0);
});

test("an unlisted artifact downloaded with no name is still a handoff", async () => {
  const { code, err } = await run(releaseWorkflow("report", 7, "          path: artifacts"));
  expect(code).toBe(1);
  expect(err).toContain("run-scoped handoff allows at most 1");
});

test("a retention cap that is an expression is reported as non-literal", async () => {
  const { code, err } = await run(`${HEADER}      - uses: actions/upload-artifact@v4
        with:
          name: report
          path: out/
          retention-days: \${{ inputs.days }}
`);
  expect(code).toBe(1);
  expect(err).toContain("not a literal day count");
});

test("a cache key built from github.sha fails", async () => {
  const { code, err } = await run(`${HEADER}      - uses: actions/cache@v4
        with:
          path: ~/.npm
          key: npm-\${{ github.sha }}
`);
  expect(code).toBe(1);
  expect(err).toContain("unique per commit");
});

test("a cache key built from github.run_id fails", async () => {
  const { code, err } = await run(`${HEADER}      - uses: actions/cache@v4
        with:
          path: ~/.npm
          key: npm-\${{ github.run_id }}
`);
  expect(code).toBe(1);
  expect(err).toContain("unique per run");
});

test("a composite action is scanned for retention caps", async () => {
  const { code, err } = await runTree({
    ".github/actions/upload/action.yml": `name: upload
description: sample
runs:
  using: composite
  steps:
    - uses: actions/upload-artifact@v4
      with:
        name: report
        path: out/
        retention-days: 8
`,
  });
  expect(code).toBe(1);
  expect(err).toContain(".github/actions/upload/action.yml");
  expect(err).toContain("default cap allows at most 7");
});

test("a capped composite action passes", async () => {
  const { code, err } = await runTree({
    ".github/actions/upload/action.yml": `name: upload
description: sample
runs:
  using: composite
  steps:
    - uses: actions/upload-artifact@v4
      with:
        name: report
        path: out/
        retention-days: 7
`,
  });
  expect(err).toBe("");
  expect(code).toBe(0);
});

test("a repository without composite actions still passes", async () => {
  const { code, err } = await runTree({ "ci.yml": CAPPED }, (root) => [root, join(root, ".github/actions")]);
  expect(err).toBe("");
  expect(code).toBe(0);
});
