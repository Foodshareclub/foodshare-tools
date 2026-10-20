#!/usr/bin/env bun
/**
 * Xcode Cloud Log Streamer & Diagnostic Integration — now a Rust binary.
 *
 * The App Store Connect integration lives in `foodshare-xccloud`; this
 * script is the shim that keeps the sibling repositories working while
 * they are updated.
 *
 * Usage: bun tools/xccloud.ts [status|start|watch|logs|jwt|local]
 *
 * @module tools/xccloud
 */

import { runRustBinAndExit } from "./rust-bin.ts";

await runRustBinAndExit("fs-xccloud", process.argv.slice(2));
