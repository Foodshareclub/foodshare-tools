#!/usr/bin/env bun
/**
 * FoodShare Universal Mobile Runner & Builder — now a Rust binary.
 *
 * The simulator flow lives in `fs-app`; this script is the shim that
 * keeps the sibling repositories working while they are updated.
 *
 * Usage: bun tools/sim-runner.ts [--clean|--smoke|--test|--no-build|--device=<udid>]
 *
 * @module tools/sim-runner
 */

import { runRustBinAndExit } from "./rust-bin.ts";

await runRustBinAndExit("fs-sim", process.argv.slice(2));
