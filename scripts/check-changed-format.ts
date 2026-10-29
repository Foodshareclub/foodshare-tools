// Adopt Bun formatting on changed source without rewriting unrelated files.
import { existsSync } from "bun:fs";

function git(args: string[], input?: string): string {
  const p = Bun.spawnSync(["git", ...args], {
    stdin: input === undefined ? "ignore" : Buffer.from(input),
    stdout: "pipe",
    stderr: "pipe",
  });
  if (p.exitCode !== 0) throw new Error(`Cannot determine formatting scope: git ${args[0]} failed`);
  return p.stdout.toString();
}
function resolves(ref: string): boolean {
  return (
    Bun.spawnSync(["git", "rev-parse", "--verify", "--end-of-options", ref], { stdout: "ignore", stderr: "ignore" })
      .exitCode === 0
  );
}
let base = process.env.FORMAT_BASE_REF;
if (!base && process.env.GITHUB_ACTIONS === "true") {
  const path = process.env.GITHUB_EVENT_PATH;
  if (!path) throw new Error("CI must provide GITHUB_EVENT_PATH or FORMAT_BASE_REF");
  const event = await Bun.file(path).json();
  base = event.pull_request?.base?.sha ?? event.before ?? "HEAD~1";
}
base ??= "HEAD";
if (/^0+$/.test(base)) base = git(["hash-object", "-w", "-t", "tree", "--stdin"], "").trim();
if (!resolves(base)) {
  if (process.env.GITHUB_ACTIONS !== "true") throw new Error("Formatting base is unavailable");
  const target = /^[a-f0-9]{40}$/.test(base) ? base : git(["rev-parse", "HEAD"]).trim();
  git(["fetch", "--no-tags", "--depth=2", "origin", target]);
  if (!resolves(base)) throw new Error("CI formatting base is unavailable after fetch");
}
const committed = process.env.GITHUB_ACTIONS === "true" || process.env.FORMAT_COMMITTED_ONLY === "true";
const files = new Set([
  ...git(["diff", "--name-only", "-z", "--diff-filter=ACMR", base, ...(committed ? ["HEAD"] : []), "--"]).split("\0"),
  ...(committed ? [] : git(["ls-files", "--others", "--exclude-standard", "-z"]).split("\0")),
]);
const config = (existsSync("bunfig.toml") ? Bun.TOML.parse(await Bun.file("bunfig.toml").text()) : {}) as {
  fmt?: { ignorePatterns?: string[] };
};
const patterns = config.fmt?.ignorePatterns as string[] | undefined;
const ignored = (patterns ?? []).map((pattern) => new Bun.Glob(pattern));
const sources = [...files]
  .filter((file) => file && /\.[cm]?[jt]sx?$|\.(json|css)$/.test(file))
  .filter((file) => !/^(node_modules|supabase|tools|artifacts|public|\.build|\.next)\/|^src\/wasm\//.test(file))
  .filter((file) => existsSync(file) && !ignored.some((glob) => glob.match(file)));
if (!sources.length) {
  console.log("No changed JavaScript, TypeScript, JSON or CSS files to format");
} else {
  const p = Bun.spawn([process.execPath, "fmt", "--no-script", "--check", "--", ...sources], {
    stdout: "inherit",
    stderr: "inherit",
  });
  process.exit(await p.exited);
}
