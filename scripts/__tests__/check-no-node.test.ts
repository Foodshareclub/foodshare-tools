import { describe, expect, it } from "bun:test";

/**
 * The Node.js guard decides what counts as a Node.js dependency in a
 * repository that runs on Bun only. It is the check that keeps Node out,
 * and it is nearly a thousand lines of rules about where a Node builtin
 * may be mentioned and where it may not.
 *
 * A guard that is never tested is a guard that can be wrong in both
 * directions at once: it can let a real Node dependency through, and it
 * can report a violation in a file that has none. The second is worse,
 * because a guard that cries wolf gets ignored, and then it protects
 * nothing.
 *
 * So the rules are tested here directly. The guard is a script, not a
 * module, so the parts that are worth testing are the ones that can be
 * read out of it: the table of rewrites, the set of bare builtins, and
 * the decision about which files are scanned at all.
 *
 * `scripts/check-bun-modules.ts` is the other half of this: it checks
 * that the guard's advice and the type declarations agree, so that code
 * which follows the guard's advice can be type-checked.
 */

// ---------------------------------------------------------------------------
// The guard's rules, read out of the script itself.
//
// The script is not a module, so it cannot be imported. Reading the two
// tables out of its source is what a test of a script can do, and it
// keeps the test honest: if the guard's table changes and this test
// does not, the test fails, which is the point.
// ---------------------------------------------------------------------------

const GUARD_SOURCE = await Bun.file(
	new URL("../check-no-node.ts", import.meta.url),
).text();

/** The guard's table of `node:` builtin to `bun:` spelling. */
function guardTable(): Map<string, string> {
	const block = GUARD_SOURCE.match(
		/const BUN_MODULE_FOR[^=]*=\s*\{([^}]*)\}/,
	)?.[1];
	if (!block) throw new Error("the guard has no BUN_MODULE_FOR table");
	const table = new Map<string, string>();
	for (const [, node, bun] of block.matchAll(/"([^"]+)":\s*"([^"]+)"/g)) {
		table.set(node, bun);
	}
	return table;
}

/** The builtins the guard allows to be imported without a prefix. */
function bareBuiltins(): Set<string> {
	const block = GUARD_SOURCE.match(
		/const BARE_NODE_BUILTINS = new Set\(\[([^\]]*)\]/,
	)?.[1];
	if (!block) throw new Error("the guard has no BARE_NODE_BUILTINS set");
	return new Set(
		[...block.matchAll(/"([^"]+)"/g)].map(([, name]) => name),
	);
}

/** The directories the guard scans. */
function scannedDirs(): string[] {
	const block = GUARD_SOURCE.match(
		/const SCANNED_DIRS = \[([^\]]*)\]/,
	)?.[1];
	if (!block) throw new Error("the guard has no SCANNED_DIRS list");
	return [...block.matchAll(/"([^"]+)"/g)].map(([, dir]) => dir);
}

describe("the Node.js guard's table", () => {
	it("rewrites the builtins the tree actually uses", () => {
		const table = guardTable();
		// These are the spellings the guard has to offer, because
		// `check-bun-modules.ts` maps every one of them to a `bun:`
		// spelling that the type declarations have to declare.
		for (const node of [
			"node:path",
			"node:fs",
			"node:fs/promises",
			"node:os",
			"node:crypto",
			"node:util",
			"node:dns/promises",
		]) {
			expect(table.has(node)).toBe(true);
		}
	});

	it("rewrites every builtin to a spelling Bun implements", () => {
		const table = guardTable();
		for (const [node, bun] of table) {
			// The `node:` prefix is replaced, never dropped: a mapping
			// to something that is not the same module would send the
			// reader to a different API.
			expect(bun.startsWith("bun:")).toBe(true);
			expect(bun.slice("bun:".length)).toBe(
				node.slice("node:".length),
			);
		}
	});

	it("does not map anything twice", () => {
		const table = guardTable();
		const seen = new Set<string>();
		for (const bun of table.values()) {
			expect(seen.has(bun)).toBe(false);
			seen.add(bun);
		}
	});
});

describe("the Node.js guard's bare builtins", () => {
	it("lists the builtins the tree imports without a prefix", () => {
		const bare = bareBuiltins();
		for (const name of ["fs", "path", "os", "crypto", "util", "process"]) {
			expect(bare.has(name)).toBe(true);
		}
	});

	it("does not list a prefixed spelling, which the table handles", () => {
		const bare = bareBuiltins();
		for (const name of bare) {
			expect(name.startsWith("node:")).toBe(false);
			expect(name.startsWith("bun:")).toBe(false);
		}
	});
});

describe("the Node.js guard's scan", () => {
	it("scans the directories that hold this repository's own code", () => {
		const dirs = scannedDirs();
		for (const dir of ["scripts", "types", ".github/workflows"]) {
			expect(dirs).toContain(dir);
		}
	});

	it("scans the standalone application, which has its own toolchain", () => {
		// The application under `crates/web` carries its own lockfile
		// and its own dependencies, so it is a separate tree that the
		// guard has to reach into on its own.
		expect(scannedDirs()).toContain("crates/web");
	});
});
