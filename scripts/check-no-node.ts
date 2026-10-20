#!/usr/bin/env bun
// Guard this project's toolchain against Node.js creeping back in.
//
// The project runs on Bun end to end: Bun is the package manager, the script
// runner, the TypeScript executor, the test runner and the type checker.
// Three things silently reintroduce Node:
//
//   1. `bun run <binary>` — without `[run] bun = true` in `bunfig.toml`, Bun
//      executes `node_modules/.bin/<binary>` by honouring the
//      `#!/usr/bin/env node` shebang that every JavaScript CLI in the
//      ecosystem ships. The process is then Node — which is not installed, so
//      the command dies with "command not found" (or, worse, silently runs on
//      a different runtime than the rest of the toolchain assumes).
//
//   2. `bunx <tool>` — unlike `bun run <script>` and `bun <bin>`, `bunx` does
//      NOT honour bunfig.toml's `[run] bun = true`. It reads the shebang of
//      the installed package instead, so `bunx eslint` executes ESLint on
//      Node. `bunx --bun <tool>` forces Bun's runtime.
//
//   3. An explicit `node`/`npm`/`npx` invocation.
//
//   4. A `node:`-prefixed import, or a bare Node builtin import, in code that
//      only ever runs on Bun. See BUN_ONLY_DIRS.
//
// Deliberately NOT flagged:
//
//   - `deno`. The Supabase Edge Functions run on Deno's runtime — that is a
//     platform constraint, not a Node dependency.
//
//   - `NODE_ENV` and other environment variable names.
//
//   - `bunx` invocations of packages that ship platform-native binaries and
//     therefore never execute on Node. These are listed in NATIVE_BUNX.
//
// Usage: bun scripts/check-no-node.ts

const NATIVE_BUNX = [
  // Biome ships a Rust binary per platform; the npm package is only a
  // launcher, so `bunx @biomejs/biome` never touches Node.
  "@biomejs/biome",
  // Oxlint is a Rust binary too.
  "oxlint",
];

const NODE_TOOLS = ["node", "npm", "npx", "pnpm", "yarn"];

/**
 * Code that only ever executes on Bun, and therefore names Bun's own modules.
 *
 * `bun:fs`, `bun:path`, `bun:os` and `bun:crypto` are Bun's spellings of the
 * standard library modules — the same API, resolved by Bun rather than Node.
 * In code that only ever runs on Bun, naming Bun's modules states that
 * dependency instead of implying a Node one.
 */
const BUN_ONLY_DIRS = ["scripts", "tools", "types", "supabase/oracle", "packages"];

/**
 * The one place Deno is allowed.
 *
 * The Supabase Edge Functions run on the Supabase Edge Runtime, which *is*
 * Deno: `Deno.serve`, `Deno.env` and `Deno.test` are load-bearing there and
 * cannot be ported. `packages/functions` is the wrapper that tests them, and
 * `volumes/functions` is a symlink to the same tree.
 *
 * Everywhere else, Deno is a second runtime where there should be one — and
 * one that is not installed on the runner that builds this project's own
 * code. The oracle, the scripts and the packages all run on Bun.
 */
const DENO_ALLOWED_DIRS: string[] = [];

/** True when the file is one of the edge functions, where Deno is the runtime. */
function isDenoAllowed(file: string): boolean {
  return DENO_ALLOWED_DIRS.some((dir) => file.startsWith(dir));
}

/**
 * The code on a line, with any trailing comment removed.
 *
 * A comment that mentions `Deno.serve` is describing the edge functions, not
 * depending on the Deno runtime, so the Deno rule reads the code and not the
 * prose around it.
 */
function codeOnLine(line: string): string {
  const trimmed = line.trim();
  if (trimmed.startsWith("//") || trimmed.startsWith("*") || trimmed.startsWith("/*")) return "";
  const comment = line.indexOf("//");
  return comment === -1 ? line : line.slice(0, comment);
}

/** The `bun:` module that replaces each `node:` builtin. */
const BUN_MODULE_FOR: Record<string, string> = {
  "node:path": "bun:path",
  "node:fs": "bun:fs",
  "node:fs/promises": "bun:fs/promises",
  "node:os": "bun:os",
  "node:crypto": "bun:crypto",
  "node:util": "bun:util",
  "node:dns/promises": "bun:dns/promises",
};

/**
 * Node builtins that may be imported without a prefix.
 *
 * A bare `import ... from "fs"` resolves to the Node builtin under `tsc` and
 * to the same module under Bun, but it reads as a Node dependency and is the
 * form the `node:`-prefixed rule exists to catch.
 */
const BARE_NODE_BUILTINS = new Set([
  "assert",
  "buffer",
  "child_process",
  "cluster",
  "crypto",
  "dns",
  "dgram",
  "events",
  "fs",
  "http",
  "https",
  "inspector",
  "module",
  "net",
  "os",
  "path",
  "perf_hooks",
  "process",
  "punycode",
  "querystring",
  "readline",
  "repl",
  "stream",
  "string_decoder",
  "timers",
  "tty",
  "url",
  "util",
  "v8",
  "vm",
  "wasi",
  "worker_threads",
  "zlib",
]);

const ROOT_FILES = ["package.json", "bunfig.toml", "lefthook.yml", "tsconfig.json"];

const SCANNED_DIRS = ["scripts", "tools", "types", "supabase/oracle", "packages", ".github/workflows"];

const SCAN_EXTENSIONS = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".sh", ".yml", ".yaml", ".json", ".toml"];

const SCRIPT_EXTENSIONS = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

type Violation = { file: string; line: number; text: string; reason: string };

/** Strip a whole-line comment so prose about Node is not mistaken for a command. */
function isCommentLine(line: string): boolean {
  const trimmed = line.trimStart();
  return trimmed.startsWith("#") || trimmed.startsWith("//") || trimmed.startsWith("*");
}

/** True when the token at `index` sits where a shell would start a command. */
function isCommandPosition(line: string, index: number, isYaml = false): boolean {
  const before = line.slice(0, index).trimEnd();
  if (before === "") return true;
  // In YAML the command can sit directly behind a key — `run: npx foo` — so a
  // trailing colon is the only thing between the key and the command. Without
  // this, every inline `run:` step in a workflow slips past the guard.
  if (isYaml && before.endsWith(":")) return true;
  return (
    before.endsWith("&&") ||
    before.endsWith("||") ||
    before.endsWith(";") ||
    before.endsWith("|") ||
    before.endsWith("$(") ||
    before.endsWith("`") ||
    before.endsWith("[") ||
    before.endsWith('["') ||
    before.endsWith("['") ||
    before.endsWith('-c "') ||
    before.endsWith("-c '")
  );
}

/**
 * True when the token is part of an expression rather than a command.
 *
 * `node = value`, `node.foo`, `arr[node]` and `node: Type` are all ordinary
 * TypeScript — the identifier happens to be called "node" because these files
 * walk AST nodes. Only a token followed by an argument is a command.
 */
function isExpressionToken(line: string, index: number, length: number): boolean {
  const rest = line.slice(index + length);
  const next = rest.trimStart()[0];
  if (next === undefined) return true; // end of line
  return "=.,:;)]}>?".includes(next);
}

/** The package a `bunx` invocation refers to, without its version specifier. */
function stripVersion(pkg: string): string {
  const at = pkg.lastIndexOf("@");
  if (at > 0) return pkg.slice(0, at);
  return pkg;
}

/**
 * Parse the text following a `bunx` token into its package and whether Bun's
 * runtime is forced.
 *
 * Two shapes occur in this repo: the shell form (`bunx --bun <pkg> …`) and
 * the spawn form (`Bun.spawn(["bunx", "--bun", <pkg>, …])`).
 */
function parseBunx(rest: string): { forcedBun: boolean; pkg: string } {
  // Spawn form: ["bunx", "--bun", <pkg>, …]
  const array = rest.match(/^["']?\s*,?\s*((?:\s*["'][^"']*["']\s*,?)+)/);
  if (array?.[1]) {
    const items = [...array[1].matchAll(/["']([^"']*)["']/g)].map((match) => match[1] ?? "");
    for (const item of items) {
      if (item.startsWith("-")) {
        if (item === "--bun") return { forcedBun: true, pkg: "" };
        continue;
      }
      return { forcedBun: false, pkg: stripVersion(item) };
    }
    return { forcedBun: false, pkg: "" };
  }

  // Shell form: bunx [--flags…] <pkg> [args…]
  let forcedBun = false;
  for (const token of rest.trim().split(/\s+/)) {
    if (!token) continue;
    if (token.startsWith("-")) {
      if (token === "--bun") forcedBun = true;
      continue;
    }
    return { forcedBun, pkg: stripVersion(token.replace(/^["']|["']$/g, "")) };
  }
  return { forcedBun: false, pkg: "" };
}

function isNativeBunx(pkg: string): boolean {
  return NATIVE_BUNX.some((allowed) => pkg === allowed || pkg.startsWith(`${allowed}-`));
}

/**
 * Every place on the line where a Node tool could be launched.
 *
 * Two sources: bare tokens in command position (shell scripts, YAML hook
 * commands, Dockerfile directives) and string literals that begin with a
 * Node tool (TypeScript that spawns a command).
 */
function commandCandidates(
  line: string,
  isScript: boolean,
  isYaml = false,
): { tool: string; rest: string; lineRest: string }[] {
  const candidates: { tool: string; rest: string; lineRest: string }[] = [];

  for (const tool of [...NODE_TOOLS, "bunx"]) {
    const pattern = new RegExp(`(^|[^-\\w])(${tool})(?=\\s|$|["'])`, "g");
    for (const match of line.matchAll(pattern)) {
      const index = match.index + match[1].length;
      if (!isCommandPosition(line, index, isYaml)) continue;
      if (isExpressionToken(line, index, tool.length)) continue;
      candidates.push({ tool, rest: line.slice(index + tool.length), lineRest: "" });
    }
  }

  if (isScript) {
    for (const match of line.matchAll(/(["'`])((?:\\.|(?!\1)[^\\])*)\1/g)) {
      const content = match[2] ?? "";
      // A command has arguments. A bare tool name in a string is a type
      // literal (`type: "node" | "way"`), not an invocation.
      const lead = content.match(/^\s*(node|npm|npx|pnpm|yarn|bunx)(?=\s+\S)/);
      if (!lead?.[1]) continue;
      const tool = lead[1];
      // `node:fs` and friends are module specifiers, not the Node runtime.
      if (tool === "node" && content.slice(lead[0].length).startsWith(":")) continue;
      candidates.push({
        tool,
        rest: content.slice(lead[0].length),
        // The package of a spawn form ("bunx" as its own array element) only
        // appears in the next literal, so keep the rest of the line too.
        lineRest: line.slice(match.index + match[0].length),
      });
    }
  }

  return candidates;
}

/** True when the file lives in a directory that only ever runs on Bun. */
function isBunOnlyFile(file: string): boolean {
  return BUN_ONLY_DIRS.some((dir) => file.startsWith(`${dir}/`));
}

/**
 * The character offsets at which template literals open and close.
 *
 * A template literal holds *data*, not code that this file runs. The WASM
 * build engine, for example, emits a loader as a string; that loader is
 * source for a different project (the web app), and the web app has its own
 * policy about which builtin spellings its own source may use. Flagging the
 * generated text would be flagging another project's code.
 *
 * The template can span several lines, so the scan runs over the whole file
 * and records every backtick that opens or closes one. Escaped backticks
 * (`` \` ``) are skipped.
 */
function findTemplateLiteralSpans(text: string): number[] {
  const spans: number[] = [];
  for (let i = 0; i < text.length; i++) {
    if (text[i] === "\\") {
      i++; // skip the escaped character
      continue;
    }
    if (text[i] === "`") spans.push(i);
  }
  return spans;
}

/** True when `index` falls between an opening and a closing backtick. */
function isInsideTemplateLiteral(templateSpans: number[], index: number): boolean {
  // The spans alternate: open, close, open, close. An odd count of spans
  // before `index` means the position is inside a template.
  let count = 0;
  for (const span of templateSpans) {
    if (span >= index) break;
    count++;
  }
  return count % 2 === 1;
}

/**
 * Node builtins imported by a Bun-only file.
 *
 * Two shapes matter: the prefixed `node:fs` and the bare `fs`. Both name a
 * Node builtin module, and in code that only ever runs on Bun the `bun:`
 * spelling is the native one.
 */
function nodeBuiltinImports(line: string, templateSpans: number[], startsInsideTemplate: boolean): string[] {
  const found: string[] = [];
  const pattern = /(?:from\s+|import\s+|require\(\s*)["']((?:node:)?[a-z_][a-z0-9_]*(?:\/[a-z_]+)*)["']/g;
  for (const match of line.matchAll(pattern)) {
    const specifier = match[1];
    if (!specifier) continue;
    // A specifier inside a template literal is generated source for another
    // project, not an import this file performs. The template may have opened
    // on an earlier line, in which case the whole line is template text.
    if (startsInsideTemplate || isInsideTemplateLiteral(templateSpans, match.index)) continue;
    if (specifier.startsWith("node:")) {
      found.push(specifier);
      continue;
    }
    const root = specifier.split("/")[0] ?? "";
    if (root && BARE_NODE_BUILTINS.has(root)) found.push(specifier);
  }
  return found;
}

function checkLine(
  file: string,
  lineNumber: number,
  line: string,
  violations: Violation[],
  templateSpans: number[],
  startsInsideTemplate: boolean,
): void {
  if (isCommentLine(line)) return;

  const isScript = SCRIPT_EXTENSIONS.some((extension) => file.endsWith(extension));
  const isYaml = /\.(ya?ml)$/.test(file);
  const seen = new Set<string>();

  for (const candidate of commandCandidates(line, isScript, isYaml)) {
    if (candidate.tool !== "bunx") {
      const key = `${candidate.tool}:${candidate.rest.slice(0, 40)}`;
      if (seen.has(key)) continue;
      seen.add(key);
      violations.push({
        file,
        line: lineNumber,
        text: line.trim(),
        reason: `\`${candidate.tool}\` invokes the Node.js toolchain — use Bun (\`bun\`, \`bunx --bun\`, \`bun run\`)`,
      });
      continue;
    }

    let { forcedBun, pkg } = parseBunx(candidate.rest);
    if (!pkg && !forcedBun && candidate.lineRest) {
      const fromLine = parseBunx(candidate.lineRest);
      pkg = fromLine.pkg;
      forcedBun = fromLine.forcedBun;
    }
    if (forcedBun) continue;
    if (pkg && isNativeBunx(pkg)) continue;

    const key = `bunx:${pkg}`;
    if (seen.has(key)) continue;
    seen.add(key);
    violations.push({
      file,
      line: lineNumber,
      text: line.trim(),
      reason: pkg
        ? `\`bunx ${pkg}\` runs on Node — use \`bunx --bun ${pkg}\`, or \`bun node_modules/<pkg>/bin/…\` for a local dependency`
        : "`bunx` without `--bun` runs on Node — use `bunx --bun <tool>`",
    });
  }

  // A `#!/usr/bin/env node` shebang pins the file to the Node runtime.
  if (/^#!\s*\/usr\/bin\/env\s+node\b/.test(line.trim())) {
    violations.push({
      file,
      line: lineNumber,
      text: line.trim(),
      reason: "node shebang — use `#!/usr/bin/env bun`",
    });
  }

  // A workflow that installs Node is a Node toolchain, however it is spelled.
  if (/actions\/setup-node\b/.test(line)) {
    violations.push({
      file,
      line: lineNumber,
      text: line.trim(),
      reason: "`actions/setup-node` installs a Node.js toolchain — this project runs on Bun",
    });
  }

  // Deno is a second runtime, and one that is not installed on the runner
  // that builds this project's own code. It is allowed in exactly one place:
  // the Supabase Edge Functions, which run on the Edge Runtime.
  //
  // Only source is checked. A workflow that *runs* `deno` is running the edge
  // functions — the jobs that do it set `working-directory: supabase/functions`
  // — and a report that prints the name of an API is not a dependency on it.
  if (!isDenoAllowed(file) && !isYaml && !isScript && /\bDeno\.[a-zA-Z]/.test(codeOnLine(line))) {
    violations.push({
      file,
      line: lineNumber,
      text: line.trim(),
      reason:
        "`Deno.*` is the Deno runtime — it belongs only in `supabase/functions/`, which runs on the Supabase Edge Runtime",
    });
  }

  // `engines.node` in package.json is a promise that the project runs on Node.
  if (file === "package.json" && /"node"\s*:/.test(line)) {
    violations.push({
      file,
      line: lineNumber,
      text: line.trim(),
      reason: "`engines.node` declares a Node.js requirement — this project runs on Bun",
    });
  }

  // Bun-only code names Bun's modules, not Node's.
  if (isBunOnlyFile(file)) {
    for (const specifier of nodeBuiltinImports(line, templateSpans, startsInsideTemplate)) {
      const key = `builtin:${specifier}`;
      if (seen.has(key)) continue;
      seen.add(key);
      const replacement = specifier.startsWith("node:")
        ? (BUN_MODULE_FOR[specifier] ?? specifier.replace(/^node:/, "bun:"))
        : `bun:${specifier}`;
      violations.push({
        file,
        line: lineNumber,
        text: line.trim(),
        reason: `\`${specifier}\` is a Node builtin — this file only runs on Bun, so use \`${replacement}\``,
      });
    }
  }
}

async function collectFiles(): Promise<string[]> {
  const files: string[] = [];

  for (const file of ROOT_FILES) {
    if (await Bun.file(file).exists()) files.push(file);
  }

  for (const dir of SCANNED_DIRS) {
    const glob = new Bun.Glob(`${dir}/**/*`);
    for await (const path of glob.scan({ cwd: "." })) {
      if (!SCAN_EXTENSIONS.some((extension) => path.endsWith(extension))) continue;
      files.push(path);
    }
  }

  return files.sort();
}

/**
 * Verify `bunfig.toml` sets `[run] bun = true`.
 *
 * Without it, `bun run <binary>` executes `node_modules/.bin/<binary>` by
 * honouring the `#!/usr/bin/env node` shebang that every JavaScript CLI in
 * the ecosystem ships. The process is then Node — which is not installed, so
 * the command dies with "command not found", or, worse, silently runs on a
 * different runtime than the rest of the toolchain assumes.
 *
 * This is the single setting that decides whether the toolchain is Bun or
 * Node, and it is invisible in a diff: removing it leaves every file in the
 * repository looking correct.
 */
async function checkBunfig(): Promise<Violation[]> {
  const violations: Violation[] = [];
  const path = "bunfig.toml";

  if (!(await Bun.file(path).exists())) {
    violations.push({
      file: path,
      line: 1,
      text: "(missing)",
      reason:
        "`bunfig.toml` is missing — without `[run] bun = true`, `bun run <binary>` executes the binary's `#!/usr/bin/env node` shebang and runs it on Node",
    });
    return violations;
  }

  const text = await Bun.file(path).text();

  // Parse the TOML minimally: find the `[run]` table and look for `bun = true`
  // inside it. A full TOML parser is not worth a dependency here, and the
  // setting has exactly one legal spelling.
  const runTable = /^\s*\[run\]\s*$/m.exec(text);
  if (!runTable) {
    violations.push({
      file: path,
      line: 1,
      text: text.trim().split("\n")[0] ?? "",
      reason: "`bunfig.toml` has no `[run]` table — add `[run]` with `bun = true` so `bun run <binary>` runs on Bun",
    });
    return violations;
  }

  const afterRunTable = text.slice(runTable.index);
  // The `[run]` table ends at the next table header. `afterRunTable` starts
  // with the `[run]` line itself, so search for the next header *after* it.
  const nextTable = /^\s*\[/m.exec(afterRunTable.slice(runTable[0].length));
  const runSection = nextTable
    ? afterRunTable.slice(0, runTable[0].length + nextTable.index)
    : afterRunTable;

  if (!/^\s*bun\s*=\s*true\s*$/m.test(runSection)) {
    violations.push({
      file: path,
      line: 1,
      text: runSection.trim().split("\n")[0] ?? "",
      reason:
        "`[run] bun = true` is not set — `bun run <binary>` would honour the binary's `#!/usr/bin/env node` shebang and run it on Node",
    });
  }

  return violations;
}

const violations: Violation[] = [];
const files = await collectFiles();

for (const file of files) {
  // The guard necessarily contains the very patterns it looks for.
  if (file === "scripts/check-no-node.ts") continue;
  const text = await Bun.file(file).text();
  const templateSpans = findTemplateLiteralSpans(text);
  let offset = 0;
  for (const [index, line] of text.split("\n").entries()) {
    // The template scan works on whole-file offsets. Shift the spans into
    // this line's coordinate space, and keep the ones that opened on an
    // earlier line as "already inside" so a multi-line template is not
    // mistaken for code.
    const lineSpans: number[] = [];
    let carriedInside = false;
    for (const span of templateSpans) {
      if (span < offset) {
        // A span that closed before this line: it opened and closed here.
        continue;
      }
      if (span >= offset + line.length) break;
      lineSpans.push(span - offset);
    }
    if (isInsideTemplateLiteral(templateSpans, offset)) carriedInside = true;

    checkLine(file, index + 1, line, violations, lineSpans, carriedInside);
    offset += line.length + 1;
  }
}

violations.push(...(await checkBunfig()));

if (violations.length > 0) {
  console.error("❌ Node.js usage found in the toolchain:\n");
  for (const violation of violations) {
    console.error(`  ${violation.file}:${violation.line}`);
    console.error(`    ${violation.text}`);
    console.error(`    → ${violation.reason}\n`);
  }
  console.error(`${violations.length} violation(s) across ${files.length} scanned file(s).`);
  process.exit(1);
}

console.log(`✅ No Node.js usage in the toolchain (${files.length} file(s) scanned)`);
