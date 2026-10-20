#!/usr/bin/env bun
/**
 * Multi-Device Parallel Matrix Runner — now a Rust binary.
 *
 * The matrix runner lives in `foodshare-maestro`; this script is the shim
 * that keeps the sibling repositories working while they are updated.
 *
 * Usage: bun tools/maestro-matrix.ts [tag|flow]
 *
 * @module tools/maestro-matrix
 */

import { runRustBinAndExit } from "./rust-bin.ts";

await runRustBinAndExit("fs-maestro", ["matrix", ...process.argv.slice(2)]);
