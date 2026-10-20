#!/usr/bin/env bun
/**
 * Disk Space Health Guard & Auto-Healer — now a Rust binary.
 *
 * The disk healing lives in `foodshare-orchestrator`; this script is the
 * shim that keeps the sibling repositories working while they are updated.
 *
 * Usage: bun tools/clean-disk.ts [--force]
 *
 * @module tools/clean-disk
 */

import { runRustBinAndExit } from "./rust-bin.ts";

await runRustBinAndExit("fs-orchestrator", ["disk", ...process.argv.slice(2)]);
