/**
 * Bridge to this repository's Rust binaries.
 *
 * The orchestration moved to Rust. This shim exists so the sibling
 * repositories — which call these scripts through the `tools/` symlink —
 * keep working while they are updated. Every script that used to hold
 * the logic is now a thin wrapper around a Rust binary.
 *
 * @module tools/rust-bin
 */

import { existsSync } from "bun:fs";
import { join } from "bun:path";

/**
 * Run one of this repository's Rust binaries.
 *
 * The release build is preferred over the debug build. When neither is
 * built, `cargo run` builds it and runs it, which always works.
 *
 * @returns The exit code of the binary.
 */
export async function runRustBin(
  name: string,
  args: string[] = [],
): Promise<number> {
  const toolsDir = import.meta.dir;

  for (const profile of ["release", "debug"]) {
    const bin = join(toolsDir, "target", profile, name);
    if (existsSync(bin)) {
      const proc = Bun.spawn([bin, ...args], {
        cwd: process.cwd(),
        stdin: "inherit",
        stdout: "inherit",
        stderr: "inherit",
      });
      return await proc.exited;
    }
  }

  const proc = Bun.spawn(["cargo", "run", "--bin", name, "--", ...args], {
    cwd: toolsDir,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  return await proc.exited;
}

/**
 * Run a Rust binary and exit with its code.
 */
export async function runRustBinAndExit(
  name: string,
  args: string[] = [],
): Promise<never> {
  process.exit(await runRustBin(name, args));
}
