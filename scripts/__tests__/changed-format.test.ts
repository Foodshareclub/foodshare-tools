import { afterEach, expect, test } from "bun:test";
import { mkdtemp, rm } from "bun:fs/promises";
import { tmpdir } from "bun:os";
import { join } from "bun:path";

const roots: string[] = [];
const checker = new URL("../check-changed-format.ts", import.meta.url).pathname;
const clean = Object.fromEntries(
  Object.entries(process.env).filter(
    ([key]) => !key.startsWith("GIT_") && !key.startsWith("GITHUB_") && key !== "FORMAT_BASE_REF",
  ),
);
function command(root: string, args: string[], extra: Record<string, string> = {}) {
  return Bun.spawnSync(args, { cwd: root, env: { ...clean, ...extra }, stdout: "pipe", stderr: "pipe" });
}
function git(root: string, ...args: string[]) {
  const p = command(root, [
    "git",
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "commit.gpgsign=false",
    "-c",
    "user.name=Formatter test",
    "-c",
    "user.email=formatter@example.invalid",
    ...args,
  ]);
  if (p.exitCode) throw new Error("Fixture Git command failed");
  return p.stdout.toString().trim();
}
async function fixture() {
  const root = await mkdtemp(join(tmpdir(), "fs-format-"));
  roots.push(root);
  git(root, "init", "-q");
  await Bun.write(join(root, "sample file.ts"), "const value = 1;\n");
  git(root, "add", ".");
  git(root, "commit", "-qm", "fixture");
  return root;
}
function check(root: string, extra: Record<string, string> = {}) {
  return command(root, [process.execPath, checker], extra).exitCode;
}
afterEach(async () => {
  for (const root of roots.splice(0)) await rm(root, { recursive: true, force: true });
});

test("checks files with spaces and propagates formatter failure", async () => {
  const root = await fixture();
  await Bun.write(join(root, "sample file.ts"), "const value=2\n");
  expect(check(root)).toBe(1);
  expect(command(root, [process.execPath, "fmt", "--no-script", "sample file.ts"]).exitCode).toBe(0);
  expect(check(root)).toBe(0);
});
test("leaves vendored and third-party code outside formatting scope", async () => {
  const root = await fixture();
  await Bun.write(join(root, "supabase/vendor.ts"), "const vendor=1\n");
  await Bun.write(join(root, "node_modules/library.ts"), "const library=1\n");
  expect(check(root)).toBe(0);
});
test("CI checks the committed push range even with a clean working tree", async () => {
  const root = await fixture();
  const before = git(root, "rev-parse", "HEAD");
  await Bun.write(join(root, "sample file.ts"), "const value=3\n");
  git(root, "add", ".");
  git(root, "commit", "-qm", "unformatted push fixture");
  const event = join(root, ".git", "event.json");
  await Bun.write(event, JSON.stringify({ before }));
  expect(git(root, "status", "--porcelain")).toBe("");
  expect(check(root, { GITHUB_ACTIONS: "true", GITHUB_EVENT_PATH: event })).toBe(1);
});
test("an unavailable base fails instead of accepting an empty scope", async () => {
  const root = await fixture();
  expect(check(root, { FORMAT_BASE_REF: "missing-format-base" })).not.toBe(0);
});
test("CI ignores generated workspace files outside the committed range", async () => {
  const root = await fixture();
  const before = git(root, "rev-parse", "HEAD");
  git(root, "commit", "--allow-empty", "-qm", "clean push fixture");
  const event = join(root, ".git", "event.json");
  await Bun.write(event, JSON.stringify({ before }));
  await Bun.write(join(root, "generated.ts"), "const generated=1\n");
  expect(check(root, { GITHUB_ACTIONS: "true", GITHUB_EVENT_PATH: event })).toBe(0);
  expect(check(root)).toBe(1);
});

test("CI fetches a base commit absent from a shallow checkout", async () => {
  const source = await fixture();
  const before = git(source, "rev-parse", "HEAD");
  await Bun.write(join(source, "sample file.ts"), "const value = 2;\n");
  git(source, "add", ".");
  git(source, "commit", "-qm", "formatted push");
  const clone = await mkdtemp(join(tmpdir(), "fs-format-shallow-"));
  roots.push(clone);
  git(clone, "clone", "--depth=1", "file://" + source, ".");
  expect(command(clone, ["git", "cat-file", "-e", before + "^{tree}"]).exitCode).not.toBe(0);
  expect(check(clone, { GITHUB_ACTIONS: "true", FORMAT_BASE_REF: before })).toBe(0);
  expect(command(clone, ["git", "cat-file", "-e", before + "^{tree}"]).exitCode).toBe(0);
});
