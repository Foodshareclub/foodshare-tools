#!/usr/bin/env bun
// Check that the JSON the repository ships is JSON.
//
// Bun's own reader for `package.json` is lenient: it accepts a trailing
// comma and a comment, where a strict JSON parser does not. A file that
// only Bun can read is a file that something else will choke on — and
// something else does: the guard in the sibling repositories reads every
// `package.json` in the tree, and it crashed on a trailing comma that
// arrived with a dependency change and that nothing here noticed.
//
// This reads the JSON the repository ships with a strict parser.
//
// Usage: bun scripts/check-json.ts

import { readdirSync, readFileSync, statSync } from "bun:fs";
import { join } from "bun:path";

const ROOT = new URL("..", import.meta.url).pathname;

/** The directories that hold what a build produced, not what a person wrote. */
const SKIPPED_DIRS = new Set([
  ".build",
  ".git",
  ".next",
  "DerivedData",
  "artifacts",
  "dist",
  "node_modules",
  "target",
  "volumes",
]);

/** Every JSON file below a directory. */
function jsonFiles(directory: string): string[] {
  const found: string[] = [];

  let entries: string[];
  try {
    entries = readdirSync(directory);
  } catch {
    return found;
  }

  for (const entry of entries) {
    if (SKIPPED_DIRS.has(entry)) continue;

    const path = join(directory, entry);
    let stats;
    try {
      stats = statSync(path);
    } catch {
      continue;
    }

    if (stats.isDirectory()) {
      found.push(...jsonFiles(path));
    } else if (entry.endsWith(".json")) {
      found.push(path);
    }
  }

  return found;
}

/**
 * The files whose format is JSONC, not JSON.
 *
 * A `tsconfig.json` is JSON with comments by specification, and so is a
 * `deno.json`. Everything else that is called JSON is JSON, and a strict
 * parser is what the other tools use.
 */
const JSONC_FILES = new Set(["tsconfig.json", "jsconfig.json", "deno.json", "deno.jsonc"]);

const files = jsonFiles(ROOT).filter((file) => !JSONC_FILES.has(file.split("/").pop() ?? ""));
const failures: string[] = [];

for (const file of files) {
  try {
    JSON.parse(readFileSync(file, "utf8"));
  } catch (error) {
    failures.push(
      `${file.slice(ROOT.length)}: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

if (failures.length > 0) {
  console.error(`❌ ${failures.length} JSON file(s) did not parse:\n`);
  for (const failure of failures) {
    console.error(`   - ${failure}`);
  }
  console.error(
    "\nBun's reader for package.json accepts a trailing comma and a comment.\n" +
      "Another tool will not, and the guard in the sibling repositories\n" +
      "reads every package.json in the tree.",
  );
  process.exit(1);
}

console.log(`✅ ${files.length} JSON file(s) parsed`);
