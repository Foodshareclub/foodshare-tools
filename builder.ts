#!/usr/bin/env bun
/**
 * FoodShare Cross-Platform Mobile Builder & Test Engine — now a Rust
 * binary.
 *
 * The build engine lives in `fs-app`; this script is the shim that keeps
 * the sibling repositories working while they are updated.
 *
 * Usage: bun tools/builder.ts [options]
 *
 * @module tools/builder
 */

import { runRustBinAndExit } from "./rust-bin.ts";

await runRustBinAndExit("fs-build", process.argv.slice(2));
