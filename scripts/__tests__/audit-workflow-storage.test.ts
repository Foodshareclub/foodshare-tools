import { expect, test } from "bun:test";

import { auditablePath } from "../audit-workflow-storage";

test("selects workflow YAML from the default branch", () => {
  expect(auditablePath(".github/workflows/ci.yml")).toBe(true);
  expect(auditablePath(".github/workflows/release.yaml")).toBe(true);
});

test("selects composite actions at any depth", () => {
  expect(auditablePath(".github/actions/setup/action.yml")).toBe(true);
  expect(auditablePath(".github/actions/setup/nested/action.yaml")).toBe(true);
  expect(auditablePath(".github/actions/action.yml")).toBe(true);
});

test("rejects nested workflows and unrelated YAML", () => {
  expect(auditablePath(".github/workflows/nested/ci.yml")).toBe(false);
  expect(auditablePath(".github/workflows/ci.yml.bak")).toBe(false);
  expect(auditablePath(".github/actions/setup/readme.yml")).toBe(false);
  expect(auditablePath(".github/actions/setup/action.json")).toBe(false);
});
