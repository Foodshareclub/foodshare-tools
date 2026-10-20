#!/usr/bin/env bun
/**
 * Maestro E2E Test Suite Runner — now a Rust binary.
 *
 * The runner lives in `foodshare-maestro`; this script is the shim that
 * keeps the sibling repositories working while they are updated.
 *
 * Usage: bun tools/maestro-runner.ts [check|syntax|smoke|regression|critical|a11y|test|flow <file>|report <format>|hierarchy]
 *
 * @module tools/maestro-runner
 */

import { runRustBinAndExit } from "./rust-bin.ts";

await runRustBinAndExit("fs-maestro", process.argv.slice(2));
