#!/usr/bin/env bun
/**
 * FoodShare WebAssembly Build Engine — now a Rust binary.
 *
 * The build engine lives in `foodshare-wasm`; this script is the shim
 * that keeps the sibling repositories working while they are updated.
 *
 * Usage: bun tools/build-wasm.ts [--only <crate>] [--json]
 *
 * @module tools/build-wasm
 */

import { runRustBinAndExit } from "./rust-bin.ts";

await runRustBinAndExit("fs-wasm", process.argv.slice(2));
