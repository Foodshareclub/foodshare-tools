import { describe, expect, test } from "bun:test";

import {
  closedPrCaches,
  decodeJobLog,
  duplicateCacheGroups,
  formatBytes,
  quotaLine,
  type CacheEntry,
} from "../storage-report";

const cache = (overrides: Partial<CacheEntry>): CacheEntry => ({
  id: 1,
  ref: "refs/heads/main",
  key: "bun-lock",
  size_in_bytes: 100,
  created_at: "2026-11-20T00:00:00Z",
  last_accessed_at: "2026-11-20T00:00:00Z",
  ...overrides,
});

describe("duplicateCacheGroups", () => {
  test("groups identical keys and counts the bytes beyond the largest copy", () => {
    const groups = duplicateCacheGroups([
      cache({ id: 1, key: "next", size_in_bytes: 300 }),
      cache({ id: 2, key: "next", ref: "refs/pull/7/merge", size_in_bytes: 100 }),
      cache({ id: 3, key: "bun", size_in_bytes: 50 }),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0]?.key).toBe("next");
    expect(groups[0]?.entries.map((entry) => entry.id)).toEqual([1, 2]);
    expect(groups[0]?.totalBytes).toBe(400);
    expect(groups[0]?.wastedBytes).toBe(100);
  });

  test("returns nothing when every key is unique", () => {
    expect(duplicateCacheGroups([cache({ id: 1 }), cache({ id: 2, key: "other" })])).toEqual([]);
  });
});

describe("closedPrCaches", () => {
  test("flags merge-ref caches whose pull request is closed or merged", () => {
    const states = new Map([
      ["7", { state: "closed", merged: true }],
      ["8", { state: "open", merged: false }],
    ]);
    const caches = [
      cache({ id: 1, ref: "refs/pull/7/merge" }),
      cache({ id: 2, ref: "refs/pull/8/merge" }),
      cache({ id: 3, ref: "refs/heads/main" }),
    ];

    expect(closedPrCaches(caches, states).map((entry) => entry.id)).toEqual([1]);
  });

  test("does not guess about refs with no known pull request state", () => {
    expect(closedPrCaches([cache({ id: 1, ref: "refs/pull/9/merge" })], new Map())).toEqual([]);
  });
});

describe("decodeJobLog", () => {
  test("reads the plain-text log body the jobs endpoint currently returns", () => {
    const log = decodeJobLog(new TextEncoder().encode("step output\nArtifact storage quota has been hit.\n"));

    expect(log).toContain("Artifact storage quota has been hit.");
  });
});

describe("quotaLine", () => {
  test("finds the upload-artifact storage quota message", () => {
    const log = "Starting upload\nArtifact storage quota has been hit. Unable to upload artifact.\nDone";

    expect(quotaLine(log)).toContain("storage quota has been hit");
  });

  test("ignores upload failures that are not about the quota", () => {
    expect(quotaLine("Error: Failed to create artifact")).toBeUndefined();
    expect(quotaLine("Unable to reserve cache with key bun-lock")).toBeUndefined();
  });
});

describe("formatBytes", () => {
  test("scales to the largest unit", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KiB");
    expect(formatBytes(5 * 1024 ** 3)).toBe("5.0 GiB");
  });
});
