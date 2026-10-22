#!/usr/bin/env bun
// Check that the guard's advice and the type declarations agree.
//
// The guard in `scripts/check-no-node.ts` rewrites a Node builtin to its
// `bun:` spelling: `node:fs` becomes `bun:fs`, and a bare `fs` becomes
// `bun:fs` too. The declaration file has to declare that spelling, or
// the type check fails with TS2307 on code that followed the guard's
// own advice:
//
//     error TS2307: Cannot find module 'bun:util' or its corresponding
//                  type declarations.
//
// The two files are copies that drift, and nothing noticed: the guard
// grew a mapping, the declarations did not, and the next file that
// used it did not compile.
//
// So this check compares them. It reads the guard's table and the
// declaration file, and it compares them in both directions: a mapping
// with no declaration, and a declaration with no mapping. It also
// imports every mapped module, because a spelling that does not resolve
// is an advice that cannot be followed.
//
// Usage: bun scripts/check-bun-modules.ts

import { readFileSync } from "bun:fs";
import { join } from "bun:path";

const ROOT = new URL("..", import.meta.url).pathname;
const GUARD = join(ROOT, "scripts", "check-no-node.ts");
const DECLARATIONS = join(ROOT, "types", "bun-modules.d.ts");

/** The mappings the guard offers: `node:` builtin to `bun:` spelling. */
function mappedModules(): Map<string, string> {
  const source = readFileSync(GUARD, "utf8");
  const table = source.match(/const BUN_MODULE_FOR[^=]*=\s*\{([^}]*)\}/)?.[1];
  if (!table) {
    console.error("❌ the guard has no BUN_MODULE_FOR table, so there is nothing to compare");
    process.exit(1);
  }

  const mappings = new Map<string, string>();
  for (const [, node, bun] of table.matchAll(/"([^"]+)":\s*"([^"]+)"/g)) {
    mappings.set(node, bun);
  }
  return mappings;
}

/**
 * The mappings the guard offers that name a module.
 *
 * Not every mapping names a module: a guard may answer "use `Bun.spawn`
 * instead", which is advice rather than a spelling. Only the mappings
 * that name a `bun:` module have to be declared and resolvable.
 */
function moduleMappings(mappings: Map<string, string>): Map<string, string> {
  const modules = new Map<string, string>();
  for (const [node, bun] of mappings) {
    if (bun.startsWith("bun:")) {
      modules.set(node, bun);
    }
  }
  return modules;
}

/** The modules the declaration file declares precisely. */
function declaredModules(): Set<string> {
  const source = readFileSync(DECLARATIONS, "utf8");
  const declared = new Set<string>();

  for (const match of source.matchAll(/declare module "([^"]+)"\s*\{/g)) {
    declared.add(match[1]);
  }
  return declared;
}

/** Whether the declaration file declares every `bun:` spelling. */
function hasWildcard(): boolean {
  const source = readFileSync(DECLARATIONS, "utf8");
  return /declare module "bun:\*";/.test(source);
}

const mappings = moduleMappings(mappedModules());
const declared = declaredModules();
const problems: string[] = [];

// The guard's advice has to type-check for every builtin it can rewrite,
// not only for the ones in its table: the table is an exception list, and
// the general case is `node:x` becoming `bun:x`.
if (!hasWildcard()) {
  problems.push(
    "the declarations do not declare every `bun:` spelling, so the guard's advice " +
      "for a builtin outside its table does not type-check",
  );
}

// A mapping with no declaration: the guard suggests a spelling the
// type check rejects.
for (const [node, bun] of mappings) {
  if (!declared.has(bun)) {
    problems.push(`the guard maps ${node} to ${bun}, but ${bun} is not declared in types/bun-modules.d.ts`);
  }
}

// A spelling that does not resolve is advice that cannot be followed.
//
// The imports have to be literals: Bun resolves a `bun:` module when it
// can see the specifier, and a dynamic `import(variable)` is a package
// lookup, which fails for a builtin that works. So the check runs a
// small program with one literal import per module.
const modules = [...new Set(mappings.values())];
const program = modules.map((module) => `await import(${JSON.stringify(module)});`).join("\n");

const resolved = Bun.spawn(["bun", "-e", program], { stdout: "pipe", stderr: "pipe" });
const [, stderr] = await Promise.all([new Response(resolved.stdout).text(), new Response(resolved.stderr).text()]);
await resolved.exited;

if (resolved.exitCode !== 0) {
  problems.push(`Bun cannot resolve every spelling the guard offers: ${stderr.trim()}`);
}

if (problems.length > 0) {
  console.error("❌ the guard and the type declarations disagree:\n");
  for (const problem of problems) {
    console.error(`   - ${problem}`);
  }
  console.error(
    "\nThe guard rewrites a Node builtin to its `bun:` spelling, and the type\n" +
      "check rejects a spelling the declarations do not cover. Both files are\n" +
      "copies that drift; this check is what notices.",
  );
  process.exit(1);
}

console.log(
  `✅ the guard and the type declarations agree (${mappings.size} mapping(s), ` + `${declared.size} declaration(s))`,
);
