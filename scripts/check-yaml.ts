#!/usr/bin/env bun
// Validate YAML syntax with Bun's built-in parser.
//
// This replaces `bunx js-yaml`, which pulled a Node package from the registry
// on every hook run and executed it on the Node runtime. Bun ships a YAML 1.2
// parser (Bun.YAML) that behaves the same way js-yaml does for the GitHub
// Actions files this is pointed at — `on:` stays a string key rather than
// becoming a boolean — so the check is equivalent without the dependency.
//
// Usage: bun scripts/check-yaml.ts <file> [<file> …]

const files = Bun.argv.slice(2);

if (files.length === 0) {
  console.log("No YAML files to check");
  process.exit(0);
}

let failed = 0;

for (const file of files) {
  try {
    Bun.YAML.parse(await Bun.file(file).text());
  } catch (error) {
    failed += 1;
    console.error(`❌ ${file}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

if (failed > 0) {
  console.error(`\n${failed} of ${files.length} YAML file(s) failed to parse`);
  process.exit(1);
}

console.log(`✅ ${files.length} YAML file(s) parsed`);
