import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "bun:fs/promises";
import { tmpdir } from "bun:os";
import { join } from "bun:path";

// The guard is the only thing that stops the two storage shapes from coming
// back. A regression here would be invisible: CI would stay green while the
// rule stopped firing. These tests drive the real entry point, not a copy of
// its logic.
const checker = new URL("../check-workflow-storage.ts", import.meta.url).pathname;

const HEADER = `name: sample
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
`;

async function run(workflow: string): Promise<{ code: number | null; err: string }> {
  const root = await mkdtemp(join(tmpdir(), "storage-guard-"));
  try {
    await writeFile(join(root, "ci.yml"), workflow);
    const result = Bun.spawnSync([process.execPath, checker, root], {
      stdout: "pipe",
      stderr: "pipe",
    });
    return { code: result.exitCode, err: result.stderr.toString() };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test("a capped upload and a content-addressed cache key pass", async () => {
  const { code, err } = await run(`${HEADER}      - uses: actions/upload-artifact@v4
        with:
          name: report
          path: out/
          retention-days: 14
      - uses: actions/cache@v4
        with:
          path: ~/.npm
          key: npm-\${{ hashFiles('bun.lock') }}
`);
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
  expect(err).toContain("90-day repository default");
});

test("a retention cap outside 1..90 fails", async () => {
  const { code, err } = await run(`${HEADER}      - uses: actions/upload-artifact@v4
        with:
          name: report
          path: out/
          retention-days: 120
`);
  expect(code).toBe(1);
  expect(err).toContain("keep it between 1 and 90");
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
