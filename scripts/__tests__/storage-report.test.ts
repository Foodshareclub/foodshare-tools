import { describe, expect, test } from "bun:test";

import {
  closedPrCaches,
  decodeJobLog,
  duplicateCacheGroups,
  encodeIssueState,
  failureKey,
  formatBytes,
  formatGbHours,
  formatPercent,
  formatUsd,
  isActionsStorageSku,
  isPackagesStorageSku,
  ISSUE_TITLE,
  markdownCell,
  mergeIssueState,
  parseIssueState,
  quotaLine,
  quotaReport,
  renderIssueBody,
  renderQuotaSection,
  renderReport,
  renderResolvedIssueBody,
  syncIssue,
  type BillingSummaryItem,
  type BillingUsageItem,
  type CacheEntry,
  type Issue,
  type IssueApi,
  type QuotaFailure,
  type QuotaReport,
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

const usage = (overrides: Partial<BillingUsageItem>): BillingUsageItem => ({
  date: "2026-11-01T00:00:00Z",
  product: "actions",
  sku: "Actions storage",
  quantity: 1,
  unitType: "GigabyteHours",
  repositoryName: "foodshare-app",
  ...overrides,
});

const summaryItem = (overrides: Partial<BillingSummaryItem>): BillingSummaryItem => ({
  product: "Actions",
  sku: "actions_storage",
  grossQuantity: 0,
  discountQuantity: 0,
  netQuantity: 0,
  grossAmount: 0,
  discountAmount: 0,
  netAmount: 0,
  pricePerUnit: 0.00033602,
  unitType: "gigabyte-hours",
  ...overrides,
});

const failure = (overrides: Partial<QuotaFailure> = {}): QuotaFailure => ({
  repo: "Foodshareclub/foodshare-app",
  run: "https://github.com/Foodshareclub/foodshare-app/actions/runs/123",
  job: "Build Android",
  step: "Upload debug APK",
  line: "Artifact storage quota has been hit.",
  ...overrides,
});

function fakeIssueApi(initial?: Issue) {
  const calls = {
    created: [] as Array<{ title: string; body: string }>,
    updated: [] as Array<{ number: number; body: string; state?: "open" | "closed" }>,
  };
  let current = initial;
  const api: IssueApi = {
    async find() {
      return current;
    },
    async create(input) {
      calls.created.push(input);
      current = {
        number: 1,
        title: input.title,
        state: "open",
        body: input.body,
        html_url: "https://github.com/Foodshareclub/foodshare-tools/issues/1",
      };
      return current;
    },
    async update(number, input) {
      calls.updated.push({ number, ...input });
      if (current === undefined) {
        throw new Error("no issue to update");
      }
      current = { ...current, ...input };
      return current;
    },
  };
  return {
    api,
    calls,
    get current() {
      return current;
    },
  };
}

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

describe("markdownCell", () => {
  test("escapes backslashes before pipes so neither can break the table", () => {
    expect(markdownCell("a\\b|c")).toBe("a\\\\b\\|c");
  });
});

describe("formatBytes", () => {
  test("scales to the largest unit", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KiB");
    expect(formatBytes(5 * 1024 ** 3)).toBe("5.0 GiB");
  });
});

describe("quota formatting", () => {
  test("renders GB-hours, percent and USD", () => {
    expect(formatGbHours(436.02)).toBe("436.0 GB-hours");
    expect(formatPercent(117.38)).toBe("117.4%");
    expect(formatUsd(0.021512)).toBe("$0.0215");
    expect(formatUsd(1.5)).toBe("$1.50");
    expect(formatUsd(0)).toBe("$0.00");
  });
});

describe("billing SKU normalization", () => {
  test("accepts the detailed and summary spellings", () => {
    expect(isActionsStorageSku("Actions storage")).toBe(true);
    expect(isActionsStorageSku("actions_storage")).toBe(true);
    expect(isActionsStorageSku("Actions Linux")).toBe(false);
    expect(isPackagesStorageSku("packages_storage")).toBe(true);
    expect(isPackagesStorageSku("Packages storage")).toBe(true);
    expect(isPackagesStorageSku("Actions storage")).toBe(false);
  });
});

describe("quotaReport", () => {
  test("separates public usage, counts billable usage, and finds the crossing day", () => {
    const result = quotaReport({
      summary: {
        usageItems: [
          summaryItem({
            grossQuantity: 2897.574722857,
            discountQuantity: 2833.55412806,
            netQuantity: 64.020594797,
            grossAmount: 0.973642204,
            discountAmount: 0.952130033,
            netAmount: 0.021512171,
          }),
          summaryItem({ product: "Packages", sku: "packages_storage" }),
        ],
      },
      detailed: [
        usage({ date: "2026-11-01T00:00:00Z", quantity: 154.3706 }),
        usage({ date: "2026-11-19T00:00:00Z", quantity: 221.4 }),
        usage({ date: "2026-11-20T00:00:00Z", quantity: 58.4 }),
        usage({ date: "2026-11-21T00:00:00Z", quantity: 1.85 }),
        usage({ date: "2026-11-01T00:00:00Z", quantity: 2461.55, repositoryName: "foodshare-web" }),
        usage({ date: "2026-11-01T00:00:00Z", quantity: 0.00003, repositoryName: "foodshare-backend" }),
      ],
      privateRepos: new Set(["foodshare-app", "foodshare-backend"]),
      currentArtifactBytes: 65_501_781,
      now: new Date("2026-11-21T12:00:00Z"),
    });

    if (!result.available) throw new Error(result.error);
    expect(result.actionsGbHours).toBeCloseTo(436.02, 1);
    expect(result.publicActionsGbHours).toBeCloseTo(2461.55, 1);
    expect(result.combinedGbHours).toBeCloseTo(436.02, 1);
    expect(result.allowanceGbHours).toBe(372);
    expect(result.usedPercent).toBeCloseTo(117.2, 0);
    expect(result.exhausted).toBe(true);
    expect(result.exhaustedOn).toBe("2026-11-19");
    expect(result.netGbHours).toBeCloseTo(64.02, 1);
    expect(result.accruedCost).toBeCloseTo(0.021512, 4);
    expect(result.projectedGbHours).toBeGreaterThan(result.combinedGbHours);
    expect(result.projectedCost).toBeGreaterThan(result.accruedCost);
  });

  test("projects exhaustion from current billable storage", () => {
    const result = quotaReport({
      summary: {
        usageItems: [summaryItem({ grossQuantity: 100, discountQuantity: 100, netQuantity: 0, netAmount: 0 })],
      },
      detailed: [usage({ quantity: 100 })],
      privateRepos: new Set(["foodshare-app"]),
      currentArtifactBytes: 0.2 * 1024 ** 3,
      now: new Date("2026-11-01T00:00:00Z"),
    });

    if (!result.available) throw new Error(result.error);
    expect(result.exhausted).toBe(false);
    expect(result.projectedGbHours).toBeCloseTo(244, 0);
    expect(result.projectedOverGbHours).toBe(0);
    expect(result.projectedCost).toBe(0);
    expect(result.projectedExhaustion).toBe("after month end");
  });

  test("includes Packages in the shared allowance and cost", () => {
    const result = quotaReport({
      summary: {
        usageItems: [
          summaryItem({ grossQuantity: 100, discountQuantity: 100, netQuantity: 0 }),
          summaryItem({
            product: "Packages",
            sku: "packages_storage",
            grossQuantity: 10,
            netQuantity: 10,
            netAmount: 0.00336,
          }),
        ],
      },
      detailed: [usage({ quantity: 100 })],
      privateRepos: new Set(["foodshare-app"]),
      currentArtifactBytes: 0,
      now: new Date("2026-11-01T00:00:00Z"),
    });

    if (!result.available) throw new Error(result.error);
    expect(result.packagesGbHours).toBe(10);
    expect(result.combinedGbHours).toBe(110);
    expect(result.usedPercent).toBeCloseTo(29.6, 0);
    expect(result.netGbHours).toBe(10);
    expect(result.accruedCost).toBeCloseTo(0.00336, 5);
    expect(result.exhausted).toBe(false);
  });
});

const exhaustedQuota: QuotaReport = {
  available: true,
  month: "2026-11",
  day: 21,
  daysInMonth: 30,
  actionsGbHours: 436.02,
  publicActionsGbHours: 2461.55,
  packagesGbHours: 0,
  combinedGbHours: 436.02,
  allowanceGbHours: 372,
  usedPercent: 117.2,
  netGbHours: 64.02,
  exhausted: true,
  exhaustedOn: "2026-11-19",
  currentArtifactBytes: 65_501_781,
  currentGb: 0.061,
  projectedGbHours: 450.4,
  projectedOverGbHours: 78.4,
  accruedCost: 0.0215,
  projectedCost: 0.0263,
};

describe("renderQuotaSection", () => {
  test("renders the accrued quota table", () => {
    const text = renderQuotaSection(exhaustedQuota).join("\n");

    expect(text).toContain("## Artifact quota (accrued)");
    expect(text).toContain("436.0 GB-hours");
    expect(text).toContain("Exhausted on 2026-11-19");
    expect(text).toContain("$0.0215");
  });

  test("explains a missing billing scope instead of failing", () => {
    const text = renderQuotaSection({ available: false, error: "GET usage -> 403: forbidden" }).join("\n");

    expect(text).toContain("Billing usage unavailable");
    expect(text).toContain("403");
    expect(text).toContain("ORG_BILLING_TOKEN");
  });
});

describe("renderReport", () => {
  test("places the accrued quota section between totals and cache findings", () => {
    const text = renderReport([], [], "2026-11-21T00:00:00Z", exhaustedQuota);
    const quota = text.indexOf("## Artifact quota (accrued)");

    expect(quota).toBeGreaterThan(text.indexOf("## Totals"));
    expect(quota).toBeLessThan(text.indexOf("## Duplicate caches"));
  });
});

describe("issue alert", () => {
  const now = "2026-11-21T06:10:00.000Z";

  test("round-trips the hidden state marker", () => {
    const f = failure();
    const state = {
      firstSeen: "2026-11-20T06:10:00.000Z",
      failures: {
        [failureKey(f)]: { firstSeen: "2026-11-20T06:10:00.000Z", lastSeen: now },
      },
    };

    expect(parseIssueState(encodeIssueState(state))).toEqual(state);
  });

  test("ignores a missing or malformed marker", () => {
    expect(parseIssueState("plain body")).toBeUndefined();
    expect(parseIssueState("<!-- storage-report-state: not-base64 -->")).toBeUndefined();
    expect(parseIssueState("<!-- storage-report-state: e30= -->")).toBeUndefined();
  });

  test("preserves first seen and refreshes last seen", () => {
    const first = failure();
    const second = failure({
      run: "https://github.com/Foodshareclub/foodshare-app/actions/runs/456",
      step: "Upload release AAB",
    });
    const old = "2026-11-20T06:10:00.000Z";
    const previous = {
      firstSeen: old,
      failures: { [failureKey(first)]: { firstSeen: old, lastSeen: old } },
    };

    const merged = mergeIssueState(previous, [first, second], now);

    expect(merged.firstSeen).toBe(old);
    expect(merged.failures[failureKey(first)]).toEqual({ firstSeen: old, lastSeen: now });
    expect(merged.failures[failureKey(second)]).toEqual({ firstSeen: now, lastSeen: now });
  });

  test("renders failing run links and both timestamps", () => {
    const f = failure();
    const state = mergeIssueState(undefined, [f], now);
    const body = renderIssueBody({
      failures: [f],
      state,
      quota: exhaustedQuota,
      reportRunUrl: "https://github.com/Foodshareclub/foodshare-tools/actions/runs/999",
      now,
    });

    expect(body).toContain(f.run);
    expect(body).toContain("First seen:");
    expect(body).toContain("Last seen:");
    expect(body).toContain(now);
    expect(body).toContain("Exhausted on 2026-11-19");
    expect(body).toContain("Scheduled report run");
    expect(parseIssueState(body)).toEqual(state);
  });

  test("renders a resolved body with no state marker", () => {
    const body = renderResolvedIssueBody({ now, quota: exhaustedQuota });

    expect(body).toContain("Resolved at");
    expect(body).not.toContain("storage-report-state");
  });

  test("creates one issue with a stable title and run link", async () => {
    const store = fakeIssueApi();
    const result = await syncIssue({
      failures: [failure()],
      quota: exhaustedQuota,
      now,
      api: store.api,
    });

    expect(result.action).toBe("created");
    expect(store.calls.created).toHaveLength(1);
    expect(store.calls.created[0]?.title).toBe(ISSUE_TITLE);
    expect(store.calls.created[0]?.body).toContain("https://github.com/Foodshareclub/foodshare-app/actions/runs/123");
  });

  test("updates the existing open issue and preserves first seen", async () => {
    const firstSeen = "2026-11-20T06:10:00.000Z";
    const store = fakeIssueApi();
    await syncIssue({ failures: [failure()], now: firstSeen, api: store.api });

    const result = await syncIssue({ failures: [failure()], now, api: store.api });
    const state = parseIssueState(store.current?.body ?? "");
    const key = failureKey(failure());

    expect(result.action).toBe("updated");
    expect(store.calls.updated).toHaveLength(1);
    expect(state?.firstSeen).toBe(firstSeen);
    expect(state?.failures[key]?.firstSeen).toBe(firstSeen);
    expect(state?.failures[key]?.lastSeen).toBe(now);
  });

  test("reopens a closed issue when failures recur", async () => {
    const store = fakeIssueApi({
      number: 7,
      title: ISSUE_TITLE,
      state: "closed",
      body: "Resolved.",
      html_url: "https://github.com/Foodshareclub/foodshare-tools/issues/7",
    });

    const result = await syncIssue({ failures: [failure()], now, api: store.api });

    expect(result.action).toBe("reopened");
    expect(store.calls.updated[0]?.state).toBe("open");
  });

  test("closes the open issue when clean", async () => {
    const store = fakeIssueApi();
    await syncIssue({ failures: [failure()], now: "2026-11-20T06:10:00.000Z", api: store.api });

    const result = await syncIssue({ failures: [], now, api: store.api });

    expect(result.action).toBe("closed");
    expect(store.calls.updated.at(-1)?.state).toBe("closed");
    expect(store.current?.state).toBe("closed");
    expect(store.current?.body).toContain("Resolved at");
    expect(store.current?.body).not.toContain("storage-report-state");
  });

  test("leaves a clean run with no open issue alone", async () => {
    const store = fakeIssueApi();

    const result = await syncIssue({ failures: [], now, api: store.api });

    expect(result.action).toBe("none");
    expect(store.calls.created).toHaveLength(0);
    expect(store.calls.updated).toHaveLength(0);
  });
});
