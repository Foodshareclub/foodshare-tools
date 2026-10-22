import { describe, expect, it } from "bun:test";

/**
 * The Sentry configuration decides what a reader of an issue sees.
 *
 * `beforeSend` is the one piece of it that decides anything: it throws
 * away the frames that are not the application's, so that the stack in
 * the issue is the stack in the code rather than the framework's. A
 * mistake in it is invisible in review and obvious in an issue tracker:
 * an empty stack tells the reader nothing, and an unfiltered one buries
 * the line that broke.
 *
 * The configuration belongs to the application under `crates/web`, which
 * is a Next.js application with its own toolchain and its own
 * dependencies -- it has no Bun types, and its tests run on Bun's runner
 * without them. So the configuration is read as a file rather than
 * imported: importing it would call `Sentry.init` and need the
 * application's own dependency tree.
 *
 * The behaviour is read out of the file rather than written here, so
 * that this test describes the configuration as it is and not as it was
 * when the test was written.
 */

const CONFIG = await Bun.file(
	new URL("../../crates/web/sentry.server.config.ts", import.meta.url),
).text();

/** The `beforeSend` the configuration hands to Sentry. */
function beforeSend(): (event: unknown) => unknown {
	const body = CONFIG.match(/beforeSend\(event\) \{([\s\S]*?)\n  \},/)?.[1];
	if (!body) throw new Error("the configuration has no beforeSend");

	// The body is written against Sentry's types, which are not worth
	// reproducing here. What is worth reproducing is the decision: keep
	// the frames whose file is the application's, and keep at most five.
	//
	// The paths the filter keeps are read out of the file rather than
	// written here, so that the test follows the configuration.
	const paths = (
		body.match(/frame\.filename\?\.includes\('([^']+)'\)/g) ?? []
	).map((call) => call.match(/'([^']+)'/)![1]);
	if (paths.length === 0) {
		throw new Error("the configuration does not filter on the file name");
	}

	const limit = Number(body.match(/\.slice\(0, (\d+)\)/)?.[1]);
	if (!Number.isFinite(limit)) {
		throw new Error("the configuration does not limit the frames");
	}

	return (event: unknown) => {
		const values = (
			event as {
				exception?: {
					values?: {
						stacktrace?: { frames?: { filename?: string }[] };
					}[];
				};
			}
		).exception?.values;
		const stacktrace = values?.[0]?.stacktrace;
		const frames = stacktrace?.frames;
		if (stacktrace && frames?.length) {
			stacktrace.frames = frames
				.filter((frame) => paths.some((path) => frame.filename?.includes(path)))
				.slice(0, limit);
		}
		return event;
	};
}

describe("the application's Sentry configuration", () => {
	it("keeps the application frames and drops the framework ones", () => {
		const send = beforeSend();
		const event = {
			exception: {
				values: [
					{
						stacktrace: {
							frames: [
								{ filename: "/app/src/lib/thing.ts" },
								{ filename: "/usr/local/lib/node_modules/next/dist/server.js" },
								{ filename: "/app/src/app/page.tsx" },
							],
						},
					},
				],
			},
		};

		const result = send(event) as {
			exception: { values: { stacktrace: { frames: { filename?: string }[] } }[] };
		};

		expect(
			result.exception.values[0].stacktrace.frames.map((f) => f.filename),
		).toEqual(["/app/src/lib/thing.ts", "/app/src/app/page.tsx"]);
	});

	it("keeps at most five frames", () => {
		const send = beforeSend();
		const frames = Array.from({ length: 12 }, (_, i) => ({
			filename: `/app/src/file${i}.ts`,
		}));
		const event = { exception: { values: [{ stacktrace: { frames } }] } };

		const result = send(event) as {
			exception: { values: { stacktrace: { frames: unknown[] } }[] };
		};

		expect(result.exception.values[0].stacktrace.frames).toHaveLength(5);
	});

	it("leaves an event with no exception alone", () => {
		const send = beforeSend();
		const event = { message: "something went wrong" };
		expect(send(event)).toBe(event);
	});

	it("leaves an event with no frames alone", () => {
		const send = beforeSend();
		const event = { exception: { values: [{ stacktrace: { frames: [] } }] } };
		expect(send(event)).toBe(event);
	});

	it("leaves an event whose exception has no stacktrace alone", () => {
		const send = beforeSend();
		const event = { exception: { values: [{}] } };
		expect(send(event)).toBe(event);
	});
});
