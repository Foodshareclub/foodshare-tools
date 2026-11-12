# Foodshare GitHub org — target configuration plan (decision-complete)

- **Task:** t_a707236e (architect) · **Date:** 2026-11-11
- **Scope:** the desired GitHub configuration for org `Foodshareclub` (id 106903029,
  display name "Foodshare") and its 12 repositories — GitHub Projects, issue
  templates, labels, milestones, discussions (+ categories/templates), insights/
  analytics, and permissions.
- **Status:** DESIGN ONLY. Nothing in §5 has been executed. The builder task
  t_9b0ea4f5 executes it; t_08aa5049 verifies it.
- **Ground truth inputs (all verified read-only by @verifier, cited inline):**
  - `artifacts/github-org-audit-2026-11-11-retry/REPORT.md` (t_e213d078, 17:30–17:55 PST) — 151 evidence files under `artifacts/github-org-audit-2026-11-11-retry/evidence/`
  - `artifacts/kanban-board-audit-2026-11-11/REPORT.md` (t_442b2cd3, 15:55–17:05 PST)
  - `foodshare-tools` `origin/main`: `docs/GHSYNC.md`, `crates/ghsync/src/{config,setup,push,intake}.rs` (read from `origin/main` at plan time; main was moving — re-verify at build time)
  - `~/.config/ghsync/config.toml` (read non-secret lines at plan time)

## 1. Problem statement

The Foodshareclub GitHub org has never been configured as a coherent whole: the
sync target board ("Foodshare Board") does not exist, ghsync is delivered but
unprovisioned (`project_id=""`, no state cursor), label sets are inconsistent
across repos, the mobile repo lacks the priority/security labels the other estate
repos carry, issue templates are old-style or absent, discussions are off, and
there is no decision on what an org member may do by default. An operator looking
at the org today cannot tell what the intended setup is.

This plan fixes the org's **configuration target state** and the ordered,
idempotent procedure to reach it. It does **not** design the bidirectional sync
semantics — that is the sibling architect card t_1cc489c3 (sync model +
remediation of the 18 cardless issues). The two cards meet at exactly one object:
the "Foodshare Board" Project v2 board. The board shape below is the contract;
t_1cc489c3 must consume it, not re-derive it.

## 2. Constraints found (cited)

1. **Repo inventory** — 12 repos, all default `main` (audit §2): estate 4
   (`foodshare-web` pub, `foodshare-backend` priv, `foodshare-app` priv,
   `foodshare-tools` pub), active public 2 (`supamigrate`, `motherduck-supasync`),
   dormant 3 (`foodshare-analytics` priv 2023, `foodshare-watchtower` priv 2025,
   `foodshareclub` priv = org-profile repo, created 11-11 mid-audit), archived 3
   (`foodshare-blog`, `foodshare-cloudflared`, `blog_backup`).
2. **ghsync is the designated provisioner for the board surface.**
   `docs/GHSYNC.md` (origin/main): `ghsync setup` "provision labels, project,
   fields, views (idempotent)"; `crates/ghsync/src/setup.rs:29-35` pins the
   constants: board title `Foodshare Board`, views `Board` + `Phases`,
   category `Handoffs`, label color `0ea4a5`. `setup.rs` steps: token check →
   per-repo labels (`ghsync`, `ghsync-<repo>`, `intake`) → find-or-create the
   board by title → fields Status (single-select: Backlog/In progress/Done/
   Blocked) + Assignee (**text** field) + Phase (single-select: Backlog/In
   progress/Done) → per-repo `default_milestone` find-or-create → discussions
   report + Handoffs category (only if the repo already has discussions enabled —
   **setup never enables discussions itself**, `setup.rs:295-304`) → grid views →
   write discovered ids back into config.
3. **Milestones are re-projected by ghsync.** `push.rs:142,198-202` +
   `intake.rs:207-217`: every linked/intake issue is assigned the repo's
   `default_milestone` (created if missing). The live org has exactly one
   milestone per estate repo, `Prod Readiness — 2026-11`, no due date (audit
   §4.3); the 3 open P0s (web #196, backend #10/#11) are milestoned to it
   (audit §4.1). ghsync's stack-named milestones do not exist yet (G11) — but
   `config.toml` already carries the right names (`Rust`, `Next.js 16 + Bun
   1.4.4`, `Rust + TypeScript`), so `setup` creates them with zero config change.
4. **Config is nearly ready.** `~/.config/ghsync/config.toml` (verified
   non-secret lines): 4 repos (no dead `catch-22` — that anomaly, A5, refers to
   the *example* file in the repo, not the live config), `profiles` = the full
   roster `[default, architect, reviewer, security, verifier]` (matches the
   intake-form dropdown), `project.number=1`, `project_id=""`, token present
   (fine-grained PAT, org Foodshareclub; per GHSYNC.md: Issues RW, Projects RW,
   Discussions RW, PR read, 90-day expiry). `state.json` absent (cursor never
   written).
5. **Intake form is live and protected.** `.github/ISSUE_TEMPLATE/agent-intake.yml`
   exists on `origin/main` of all 4 estate repos (audit S4; content verified from
   `foodshare-web origin/main` at plan time: phase dropdown Ad-hoc…P6, profile
   dropdown = the 5-roster, required acceptance textarea, `labels: [intake]`).
   **Must remain byte-identical** — intake is keyed to it.
6. **Current templates.** origin/main: web = `agent-intake.yml` only; backend =
   `agent-intake.yml` only; app = `agent-intake.yml` only; tools = `agent-intake.yml`
   + old-style `bug_report.md` + `feature_request.md` + `PULL_REQUEST_TEMPLATE.md`
   (content verified, reusable).
7. **Labels today** (audit §4.3 + sweep JSON): estate repos carry 19/19/17/13
   labels incl. `ghsync`, `ghsync-<repo>`, `intake`, `p0`, `prod-gate`,
   `security-rotation` (web/backend/tools) — **app lacks `p0`, `prod-gate`,
   `security`, `security-rotation`, `dependencies`**; public repos carry the 9
   GitHub defaults; archived/dormant carry 9 defaults.
8. **Repo config flags were NOT captured by the audit sweep** (the sweep's
   per-repo object has no `has_discussions`/`allow_forking`/merge-option keys).
   Current values are therefore **unknown** → every settings step below is
   read→diff→patch-diff-only (this is the idempotency mechanism).
9. **Org surface** (audit §2, §6.1, §6.2): 3 members (organicnz, Denisssska,
   Stas9364), 1 team (`Foodshare team`, closed), 0 org webhooks, 1 org runner
   (`pool-ghrunner-1`), 9 legacy Project v2 boards (last touched 2026-01…09,
   none named "Foodshare Board"), 1 repo webhook (foodshare-web → n8n, 2023,
   `insecure_ssl=1`, last delivery 200 OK). Blog already `foodshare.club`.
10. **Delivery convention** — `docs/MAIN_DELIVERY.md` (origin/main, web/backend/
    app; not present in tools): "Always deliver to main" … "normal
    fast-forward push … Verify the exact remote SHA and inspect its CI runs."
    CI runs on one busy org runner (audit §6.3); no branch protection on the
    public repos (audit §10 [CR]); private-repo protection is Pro-gated (403).
11. **Environment** — `gh` CLI is **not installed** on the build machine; the
    plan therefore uses `curl` against the REST/GraphQL APIs (what the verifier
    proved works) plus the `ghsync` binary itself. `cargo 1.99.1` is installed;
    **no `ghsync` binary exists yet** in `foodshare-tools/target/release/` (plan-
    time check). Private-repo git ops print `fatal: failed to store: -61`
    headless after successful reads (G15) — reads are fine; one-shot URL
    credentials are used for pushes (never printed, never stored).

## 3. Target state (the decisions)

### 3.0 Repo scope matrix

| Repo | Visibility | Treatment |
|---|---|---|
| foodshare-web, foodshare-backend, foodshare-app, foodshare-tools | estate | **Full** config (§3.1–3.7) |
| supamigrate, motherduck-supasync | public, active (dependabot) | **Partial**: labels §3.2, repo flags §3.6, templates §3.5; no discussions, no ghsync surface, no milestones |
| foodshareclub (org profile), foodshare-analytics, foodshare-watchtower | dormant/profile | **No change** (revisit on revival) |
| foodshare-blog, foodshare-cloudflared, blog_backup | archived | **No change** (GitHub enforces read-only on archived repos) |

Rationale: the operator stated (t_442b2cd3 comment, 17:45 PST) that
Foodshareclub "is the GH org umbrella for all of these repos" — but umbrella
status ≠ same configuration. Dormant/archived repos get no new surface (smallest
design that satisfies the goal); archived repos cannot be written anyway.

### 3.1 GitHub Projects

- **Exactly one** org-level Project v2 board, title **`Foodshare Board`**,
  `contentKind: ORGANIZATION`, created **by `ghsync setup`** (find-or-create by
  title, `setup.rs:114-178`). Not classic — no classic board exists and nothing
  requires one.
- Fields (created by setup if absent): `Status` single-select with options
  **Backlog, In progress, Done, Blocked**; `Assignee` **text** field; `Phase`
  single-select with options **Backlog, In progress, Done**
  (`setup.rs:230-269`, mapping in `config.rs:66-109`).
- Views: **`Board`** (kanban) + **`Phases`** (phases-by-Phase) (`setup.rs:371-390`).
- **The 9 legacy boards: no change.** No delete, no rename, no re-map. Deleting
  a board destroys its layout and any items pinned to it; they are unmapped
  legacy trackers (audit §6.1) and their disposition is an operator decision
  (§9), not a builder action.
- After setup, `config.toml` gains the discovered `project_id` + option ids
  (setup writes them back itself) — and `project.number` is updated from `1` to
  the board's real number (S3.4 below; the id is what ghsync uses, the number
  only appears in the URL/report).

### 3.2 Labels (no deletions anywhere — deletion is destructive; stale labels
cost nothing)

- **Estate 4 — keep all existing names; add to foodshare-app only:**
  `p0`, `prod-gate`, `security`, `security-rotation`, `dependencies`.
  Colors: mirror the same-named label's color read from `foodshare-web` at build
  time (e.g. web's `p0` color → app's `p0`); fallback `ededed` if the source
  label is missing.
- **supamigrate, motherduck-supasync — add:** `p0`, `prod-gate`, `security`,
  `security-rotation`, `announcement` (same color-mirroring rule).
- ghsync labels (`ghsync`, `ghsync-<repo>`, `intake`) already exist on all 4
  estate repos (audit run-21/22 API reads); `ghsync setup` re-checks idempotently
  and will create 0.
- **Do not** add `ghsync`/`intake`/`ghsync-<repo>` to non-estate repos: ghsync
  does not manage them, so such labels would promise a sync that does not exist.
- All other repos: no change.

### 3.3 Milestones

- **Keep** `Prod Readiness — 2026-11` on the 4 estate repos exactly as-is (no
  due date, no rename). It is the operator's release-gate milestone; the 3 P0s
  are assigned to it and stay there. ghsync never writes to it.
- **Create** (via `ghsync setup`, find-or-create, no due dates):
  `Next.js 16 + Bun 1.4.4` on foodshare-web **and** foodshare-app, `Rust` on
  foodshare-backend, `Rust + TypeScript` on foodshare-tools. These are the
  ghsync intake/projection buckets per `config.toml` — no config change needed.
- **No milestones** on public/dormant/archived repos.
- Rotation (e.g. a `2026-12` bucket or a renamed prod-readiness milestone) is an
  operator action; when it happens, the `default_milestone` values in
  `~/.config/ghsync/config.toml` must be updated in the same pass (setup only
  creates-if-missing; it never renames). Recorded as a standing rule, not a
  build step.

### 3.4 Issue templates + PR template

Per-repo target file set under `.github/` (delivered as files, S5):

| File | web | backend | app | tools | supamigrate | supasync |
|---|---|---|---|---|---|---|
| `ISSUE_TEMPLATE/config.yml` | + | + | + | + | + | + |
| `ISSUE_TEMPLATE/bug_report.yml` | + | + | + | + (replaces `bug_report.md`, which is deleted) | + | + |
| `ISSUE_TEMPLATE/feature_request.yml` | + | + | + | + (replaces `feature_request.md`, deleted) | + | + |
| `ISSUE_TEMPLATE/agent-intake.yml` | **unchanged** | **unchanged** | **unchanged** | **unchanged** | — (not added) | — |
| `PULL_REQUEST_TEMPLATE.md` | + | + | + | **unchanged** | + | + |

`config.yml` content (estate repos; `<repo>` substituted):

```yaml
blank_issue: true
contact_links:
  - name: Feature ideas
    url: https://github.com/Foodshareclub/<repo>/discussions
    about: Brainstorm ideas for <repo>.
  - name: Handoffs (agent feed)
    url: https://github.com/Foodshareclub/<repo>/discussions/categories/handoffs
    about: Review verdicts and agent handoffs are posted here automatically by ghsync.
```

`config.yml` for the two public repos (they have no discussions):

```yaml
blank_issue: true
```

`bug_report.yml` (all six):

```yaml
name: Bug report
about: Report a reproducible problem
title: "[bug] "
labels: [bug]
body:
  - type: textarea
    id: what
    attributes:
      label: What happened?
      description: A clear and concise description of the bug.
    validations:
      required: true
  - type: textarea
    id: repro
    attributes:
      label: Steps to reproduce
      placeholder: |
        1.
        2.
    validations:
      required: true
  - type: dropdown
    id: platform
    attributes:
      label: Platform
      options: [Web, iOS, Android, Backend, Other]
    validations:
      required: true
  - type: textarea
    id: logs
    attributes:
      label: Logs / output
      render: shell
```

`feature_request.yml` (all six):

```yaml
name: Feature request
about: Suggest an idea for this project
title: "[enhancement] "
labels: [enhancement]
body:
  - type: textarea
    id: problem
    attributes:
      label: Problem
      description: What is missing or painful, and what does it cost?
    validations:
      required: true
  - type: textarea
    id: solution
    attributes:
      label: Proposed solution
  - type: dropdown
    id: platform
    attributes:
      label: Platform
      options: [Web, iOS, Android, Backend, Other]
```

`PULL_REQUEST_TEMPLATE.md`: the file already on `foodshare-tools origin/main`
(content verified at plan time) is the canonical version — copy it verbatim to
the other five repos.

`agent-intake.yml` is **frozen**: S5 verifies its SHA-256 before and after each
push (acceptance A5).

### 3.5 Discussions (estate 4 only)

- `has_discussions: true` on the 4 estate repos (S1 REST PATCH; setup would
  otherwise report "disabled — discussion intake inert" and skip Handoffs,
  `setup.rs:295-304`).
- Categories per estate repo:
  - **`Handoffs`** — created by `ghsync setup` when absent (GraphQL
    `addDiscussionCategory` + REST rename, `setup.rs:328-344`). This is ghsync's
    feed target (GHSYNC.md "Handoffs discussions | Feed | kanban → GH"). No
    template (ghsync writes structured bodies; human verdicts use the
    `sync:github ` comment convention, not categories).
  - **`Announcements`** — created by S4: GraphQL `addDiscussionCategory` with
    `categoryType: ANNOUNCEMENT`, `embedded: true`, template:
    ```
    ## What changed

    ## Why it matters

    ## Links
    ```
    Find-or-create by name (idempotent).
- **Non-estate repos: discussions stay off** (decisions in GHSYNC.md gate feed
  and intake on the per-repo `discussions_enabled` flag recorded by setup;
  leaving them off means the flags record `false` and ghsync skips them with a
  logged skip — exactly the intended opt-out).

### 3.6 Repo settings (PATCH /repos/{o}/{r} — read→diff→patch-diff-only)

| Flag | estate 4 | supamigrate + supasync | others |
|---|---|---|---|
| `has_issues` | true (verify, no write expected) | true (verify) | no change |
| `has_projects` | true (verify) | no change | no change |
| `has_discussions` | **true** | false (verify no write) | no change |
| `has_wiki` | **false** | no change | no change |
| `allow_forking` | web/tools: true (verify, public); backend/app: **false** | no change (true default) | no change |
| `delete_branch_on_merge` | **true** (fixes the G4 root cause: merged `ghsync` branch still present) | **true** | no change |
| `allow_auto_merge` | **false** (CI is red org-wide, audit G9 — auto-merge must not merge red) | **false** | no change |
| `squash_merge_allowed` / `merge_commit_allowed` | true / true (verify; history uses merge commits + direct pushes) | no change | no change |
| `rebase_merge_allowed` | **false** | no change | no change |

Patch **only the fields that differ** from the GET read (keeps the write
minimal and the audit trail exact). Current values are unknown (constraint 8),
so the baseline read in S0 is load-bearing.

### 3.7 Org settings + permissions

- `PATCH /orgs/Foodshareclub` — only fields that differ:
  `default_repository_permission: "read"` (members get read by default; owner
  unaffected), `members_can_create_repositories: true` (keep — the org profile
  repo was created this way during the audit window; agents create repos),
  `blog: "foodshare.club"` (verify), `description`: keep if non-empty, else set
  `"FoodShare — community food-sharing app (web, iOS, Android) and its self-hosted backend."`.
- **Members: no role changes.** Verify (S0) and report the 3 roles (GET
  `/orgs/Foodshareclub/memberships/{login}` — classic PAT has `admin:org`).
  Changing member roles is an operator decision (§9).
- **Team `Foodshare team`: no change** (closed team, grants unverified with the
  audit token — report only).
- **Org secrets / runners / webhooks: no change.** (Webhook disposition §9.)
- **Branch protection: skipped by decision.** Private-repo protection rules are
  Pro-gated on this plan's tokens (audit §10: 403), the public repos have none,
  and the estate's operating model is direct delivery to main per
  `docs/MAIN_DELIVERY.md`. Not a gap to paper over — a plan item an operator can
  revisit on a paid plan.
- **Insights/analytics: no action by decision.** GitHub org Insights (usage,
  pulse, contributors, traffic) and repo Insights/traffic/pulse are read-only
  dashboards; GitHub exposes **no configurable "insights" or "analytics"
  settings** at org or repo level. The task item is closed by this finding —
  the verifier records it as "nothing to configure", not as a check to run.

## 4. Alternatives rejected

- **Hand-creating the board/fields/views via raw GraphQL instead of `ghsync
  setup`.** Rejected: duplicates the tool's provisioning logic, risks drift from
  the snapshot-pinned mutations in `github.rs`, and the board must exist in the
  exact shape setup's find-or-create expects anyway. Setup is find-or-create and
  idempotent by design — the raw API is used only for what setup does not own
  (repo/org flags, discussions enable, Announcements).
- **Monthly milestone scheme (`2026-11`, `2026-12`, …) replacing the stack
  buckets.** Rejected: churns the config (`default_milestone` must change every
  month or intake breaks), fights the existing `Prod Readiness — 2026-11`
  milestone and its 3 P0 assignments, and buys nothing for an org that does not
  run monthly sprints. Reversible: rotation is a documented operator action (§3.3).
- **Deleting/re-mapping the 9 legacy boards or the 2023 n8n webhook.** Rejected:
  both are destructive or operator-owned; nothing in the target state requires
  touching them. Recorded as §9 operator decisions instead.

## 5. Implementation (ordered; builder t_9b0ea4f5)

Tokens (never printed, never committed):
- `GH_PAT` = the classic PAT for `organicnz` from `~/.git-credentials`
  (line format `https://USER:TOKEN@github.com`). Used for org/repo settings,
  labels, milestones, categories (scopes per audit: `admin:org, delete_repo,
  gist, repo, workflow`).
  ```bash
  GH_PAT=$(grep -E '^https://[^@]+@github.com' ~/.git-credentials | head -1 | sed -E 's#^https://[^:]+:([^@]+)@github.com$#\1#')
  ```
- ghsync's fine-grained PAT lives in `~/.config/ghsync/config.toml` (`token=`)
  and is used only by the `ghsync` binary itself.

All curl calls: `curl -sS -H "Authorization: token $GH_PAT" -H "Accept:
application/vnd.github+json" …`; record each command, HTTP status, and response
file under `artifacts/github-org-config-<date>/evidence/` (mirror the audit's
convention; leak-scan the tree like the audit did).

### S0 — Baseline (read-only; stop on any anomaly)

1. `git fetch origin` in all 4 estate clones (read-only refs update; note the
   G15 `-61` stderr, it is harmless). Record `origin/main` SHA per repo — these
   are the push bases.
2. Org snapshot: `GET /orgs/Foodshareclub` → save; `GET
   /orgs/Foodshareclub/members` + `/memberships/{organicnz,Denisssska,Stas9364}`;
   `GET /orgs/Foodshareclub/teams`; `GET /orgs/Foodshareclub/hooks`.
3. Per repo (all 12): `GET /repos/Foodshareclub/<r>` **full object** (this
   captures the §3.6 flags the audit missed), `GET /repos/.../labels`, `GET
   /repos/.../milestones?state=all`.
4. GraphQL `hasDiscussions` + categories for the 4 estate repos:
   ```graphql
   query($owner: String!, $name: String!) {
     repository(owner: $owner, name: $name) {
       id hasDiscussions
       discussionCategories(first: 20) { nodes { id name slug } }
     }
   }
   ```
5. GraphQL board list:
   ```graphql
   query($org: String!) {
     organization(login: $org) {
       projectsV2(first: 100) { nodes { id number title updatedAt } }
     }
   }
   ```
   Expect: 9 boards, none titled `Foodshare Board`.
6. SHA-256 of `agent-intake.yml` on `origin/main` of all 4 estate repos
   (`git show origin/main:.github/ISSUE_TEMPLATE/agent-intake.yml | shasum -a 256`).
7. Token probe: `GET /user` with `GH_PAT` (expect login `organicnz`) and with
   the ghsync token (`token=` read from config.toml into a shell var; expect HTTP
   200). If either 401s → stop, report, operator rotates the PAT (defined
   escalation, not a design decision).
8. Toolchain: `cargo --version`; verify no `foodshare-tools/target/release/ghsync`
   → build: `cargo build --release -p ghsync` (cwd `foodshare-tools` after
   `git merge --ff-only origin/main` — the clone is clean and 0 ahead per audit
   §5.3; if not fast-forwardable, stop and report). Record the binary's source
   SHA (`git log -1 --format=%H`).
9. Write `baseline.json` (all of the above). **This file is the rollback source
   of truth.**

### S1 — Repo settings + enable discussions (estate 4 + public 2)

For each of the 6 repos in scope: diff the S0 GET object against §3.6; for each
differing field issue one `PATCH /repos/Foodshareclub/<r>` containing **only the
differing fields** (e.g. `{"has_discussions": true}`), with `GH_PAT`. Re-GET and
assert the new values. Expected first-run writes: estate 4 → `has_discussions`
(true), `has_wiki` (false), `delete_branch_on_merge` (true), `allow_auto_merge`
(false), `rebase_merge_allowed` (false), backend/app `allow_forking` (false) —
exact diff determined at runtime; public 2 → `delete_branch_on_merge`,
`allow_auto_merge`.

### S2 — Labels

1. Read the source colors from `foodshare-web`'s labels (S0 baseline).
2. `POST /repos/Foodshareclub/foodshare-app/labels` ×5 (`p0`, `prod-gate`,
   `security`, `security-rotation`, `dependencies`, color-mirrored, fallback
   `ededed`).
3. `POST /repos/Foodshareclub/supamigrate/labels` ×5 and
   `POST /repos/Foodshareclub/motherduck-supasync/labels` ×5 (`p0`, `prod-gate`,
   `security`, `security-rotation`, `announcement`).
Idempotency: find-or-create by name (re-check GET labels first); a second run
creates 0.

### S3 — ghsync provisioning (the board, its fields, milestones, Handoffs, views)

1. Back up the config: `cp ~/.config/ghsync/config.toml ~/.config/ghsync/config.toml.bak-$(date +%s)`.
2. Run `ghsync setup` (cwd anywhere; it reads `~/.config/ghsync/config.toml`).
   Expected first-run report: `user=<ghsync token login>`, `labels_created` = 0
   for all 4 repos, `project_created: true`, Status options 4 + Phase options 3
   ids written back, `milestones_created` = 1 per repo (4 total: `Next.js 16 +
   Bun 1.4.4` ×2, `Rust` ×1, `Rust + TypeScript` ×1), `discussion_flags` =
   `(true, …)` for all 4 (S1 already enabled discussions), `views_created` =
   `["Board", "Phases"]`.
3. **Idempotency proof:** run `ghsync setup` a second time immediately. Expected:
   `project_created: false`, all `labels_created` 0, all `milestones_created` 0,
   `views_created: []`. (GHSYNC.md: "a second run performs zero writes.")
4. Set `project.number` in config.toml to the board's real number (read via the
   §5/S0 GraphQL query, matched by title); leave everything else to setup's
   write-back. Re-verify `project_id` is non-empty and matches the GraphQL
   `node.id` of the `Foodshare Board` entry.

If setup fails mid-run (token scope, schema drift): the worst case is a board
created but fields/views incomplete — re-run setup (find-or-create resumes); if
it cannot be made to pass, roll back per §7 and escalate to the operator with
the exact error (the token is 90-day and may need extension — defined
escalation).

### S4 — Announcements categories (estate 4)

Per repo, find-or-create by name (S0 categories list): GraphQL
`addDiscussionCategory`:
```graphql
mutation($repoId: ID!) {
  addDiscussionCategory(input: {
    repositoryId: $repoId
    categoryType: ANNOUNCEMENT
    name: "Announcements"
    embedded: true
    template: "## What changed\n\n## Why it matters\n\n## Links"
  }) { discussionCategory { id name slug } }
}
```
`$repoId` = the repository node id from S0 step 4.

### S5 — Issue/PR templates (6 repos, delivered to `origin/main`)

Per repo, **fresh shallow worktree — never the estate clones** (web is 18 files
dirty, backend 34, app has 2 unpushed commits + 3 dirty, audit §5.3; the public
repos have no local clone at all):

```bash
git clone --depth 1 https://github.com/Foodshareclub/<r>.git /tmp/ghconfig-<r>
cd /tmp/ghconfig-<r>   # checked out at origin/main
# write/replace the §3.4 files (delete tools' bug_report.md + feature_request.md)
git add .github/ISSUE_TEMPLATE/ .github/PULL_REQUEST_TEMPLATE.md
git -c user.name="organicnz" -c user.email="organicnz@users.noreply.github.com" \
  commit -m "chore(templates): standardize issue templates and PR template"
git push "https://x-access-token:$GH_PAT@github.com/Foodshareclub/<r>.git" HEAD:main
```

Rules (per `docs/MAIN_DELIVERY.md`): push is fast-forward only — if rejected,
re-clone (main moved), re-apply, retry once; **never force-push, never
`git add -A`** (only the `.github` paths). Immediately verify: remote SHA
(`git ls-remote origin main` == pushed SHA) + the new CI run's state
(`GET /repos/.../actions/runs?per_page=1` — report passed/failed/queued
honestly; a queued run on the busy org runner is expected and is reported as
`pending`, not waited on). Re-run the `agent-intake.yml` SHA-256 check (A5).
One commit per repo; 6 commits total; commit message identical across repos.

### S6 — Org settings

`GET /orgs/Foodshareclub` (S0) vs §3.7; PATCH only differing fields among
`default_repository_permission`, `members_can_create_repositories`, `blog`,
`description`. Re-GET, assert.

### S7 — Final verification

Re-run every acceptance check in §6 fresh (not from S0/S1 caches), save the
post-state, diff baseline → final into `artifacts/github-org-config-<date>/REPORT.md`
with per-object before/after, and leak-scan the evidence tree (regex for
`ghp_`, `github_pat_`, `x-access-token`). Report: files, commands, exit codes,
HTTP statuses, remote SHAs + CI run states, and any INCONCLUSIVE item named as
such (protocol: a check that did not run is never PASS).

## 6. Acceptance checks (exact; verifier t_08aa5049 runs read-only)

- **A1 — Board:** GraphQL node query on the `Foodshare Board` id:
  ```graphql
  query($id: ID!) {
    node(id: $id) {
      ... on ProjectV2 {
        number title
        fields(first: 50) {
          nodes {
            id name
            ... on ProjectV2SingleSelectField { options { id name } }
          }
        }
        views(first: 50) { nodes { id name kind } }
      }
    }
  }
  ```
  PASS = exactly the fields `Status` (options Backlog/In progress/Done/Blocked),
  `Assignee` (text), `Phase` (options Backlog/In progress/Done) and views
  `Board` + `Phases`; and the §5/S0 board list still contains exactly one board
  titled `Foodshare Board`.
- **A2 — Labels:** for each of the 6 repos in scope, `GET /repos/.../labels`
  contains the §3.2 set (estate: existing ∪ additions; app now has `p0` + 4;
  public: defaults ∪ 5). No label was deleted (baseline ⊆ current, all 12 repos).
- **A3 — Milestones:** `GET /repos/.../milestones?state=all` on the 4 estate
  repos contains `Prod Readiness — 2026-11` **and** the stack milestone; the 3
  P0 issues (web #196, backend #10, #11) still report `milestone.title =
  "Prod Readiness — 2026-11"` (`GET /repos/.../issues/<n>`).
- **A4 — Discussions:** estate 4 `GET /repos/...` → `has_discussions: true`;
  GraphQL categories (S0 query) include `Handoffs` and `Announcements` per repo.
  Public/dormant/archived: unchanged vs baseline.
- **A5 — Templates:** `GET /repos/.../contents/.github/ISSUE_TEMPLATE` matches
  the §3.4 matrix per repo; `PULL_REQUEST_TEMPLATE.md` present on all six;
  tools no longer has `bug_report.md`/`feature_request.md`; the SHA-256 of
  `agent-intake.yml` on each estate repo's `origin/main` equals the S0 value.
- **A6 — Repo flags:** `GET /repos/...` on the 6 repos shows the §3.6 values.
- **A7 — Org:** `GET /orgs/Foodshareclub` shows `default_repository_permission=
  read`, `members_can_create_repositories=true`, `blog=foodshare.club`; member
  roles unchanged vs baseline; `GET /orgs/.../hooks` still empty; the foodshare-
  web n8n hook unchanged (`insecure_ssl: "1"`, `active: true`).
- **A8 — Idempotency + cursor:** the second `ghsync setup` run's report (S3.3)
  shows zero writes; `~/.config/ghsync/config.toml` has non-empty `project_id`
  + `status_option_ids`/`phase_option_ids` and no `catch-22` entry; `ghsync
  status --json` runs without a `setup-required` row.
- **A9 — Insights:** report states "no configurable insights/analytics settings
  exist at org or repo level" (finding, §3.7) — not a live check.
- **A10 — Remote SHAs:** for each of the 6 template commits, `git ls-remote`
  HEAD matches the reported SHA; CI run states reported (passed/failed/pending).

## 7. Rollback (per step, in reverse; baseline.json is the source of truth)

| Step | Rollback |
|---|---|
| S6 | Re-PATCH the org fields from baseline values (read/write both allowed with `GH_PAT`). |
| S5 | Per repo: `git revert` the template commit on main (fresh worktree at the remote SHA, commit, fast-forward push). Old-style files on tools come back because the revert undoes the deletion. |
| S4 | `DELETE /repos/.../discussions/categories/{id}` for Announcements (id from post-state). |
| S3 | GraphQL `deleteProjectV2(input: {projectId: <id>})` (destroys board + fields + options + views; the board has no items yet — no issues are affected); `DELETE /repos/.../labels/<name>` for any labels setup created (expected: 0); `DELETE /repos/.../milestones/{id}` for the 4 created; `DELETE /repos/.../discussions/categories/{id}` for Handoffs **only if setup created it** (check post-state vs baseline). Restore `config.toml` from the S3.1 backup. |
| S2 | `DELETE /repos/.../labels/<name>` for the 15 created (app 5, public 5+5). |
| S1 | Re-PATCH each repo with the baseline values for the fields written. |
| S0 | Nothing to roll back (read-only). |

Ordering note: delete the board (S3) before restoring labels/milestones so the
rollback cannot leave a half-provisioned board that a later `ghsync setup` would
adopt.

## 8. Risks and earliest signals

1. **Live mutations during the window** (the org is actively mutating — a repo
   was created mid-audit, tools main advanced 16→21 commits in one hour, G16).
   Earliest signal: a rejected fast-forward push in S5 or a S0 baseline entry
   that no longer matches at mutation time. Mitigation is built in: read
   immediately before each write, find-or-create everywhere, fast-forward-only
   pushes, no force.
2. **Token scope gap on the fine-grained ghsync PAT** (the audit documents its
   scopes as "include", not exhaustively). Earliest signal: any 403 during S3
   (first write is the board creation; labels/milestones are no-ops since they
   exist). Mitigation: `GH_PAT` (classic, `admin:org`+`repo`) is held in
   reserve for S1/S2/S4/S6 by design; S3 failure → operator extends/rotates the
   90-day PAT (defined escalation) — the classic PAT is **not** a silent
   fallback for ghsync (GHSYNC.md: the fine-grained token lives in the config
   only).
3. **CI congestion** (one busy org runner, audit G9/G8). Earliest signal:
   `queued` runs after S5 pushes. Not a gate (no branch protection), but MAIN_
   DELIVERY requires the run state to be reported, not ignored; the builder
   reports `pending` truthfully.
4. **ghsync schema drift** (the crate's GraphQL was just "aligned with the live
   schema" in the PR #4 merge, G8). Earliest signal: a GraphQL error naming a
   field during S3. Mitigation: build from the current `origin/main` SHA (S0.8
   records it); on drift, stop and report — do not patch the crate (that is a
   builder-with-approval action on the sync card, not this run).
5. **Headless git credential quirks** (G15 `-61` on store). Reads verified fine
   by both audits; pushes use one-shot URL credentials (never stored), so the
   failure mode cannot occur.

Cheapest experiment for the biggest unknown (fine-grained token scope): it is
already de-risked by construction — every S3 write is either a no-op
(labels/milestones exist) or the intended target object (the board), so a scope
failure is detectable on the first mutation and costs at most one re-provision.

## 9. Operator decisions explicitly NOT made here (recorded, non-blocking)

1. **2023 n8n webhook** on foodshare-web (`insecure_ssl=1`, target
   `n8n.foodshare.club` reachable, last delivery 200): keep or delete — audit A7
   flagged it for the operator. This plan leaves it untouched.
2. **9 legacy Project v2 boards**: archive-or-delete disposition. Left untouched.
3. **Member roles / team grants** for Denisssska and Stas9364 and the
   `Foodshare team` membership: verify-and-report only.
4. **Milestone rotation cadence** beyond §3.3's standing rule.
5. **Bot-account upgrade** for ghsync (GHSYNC.md documents the path: config
   change, no migration).

## 10. Boundary with t_1cc489c3 (sync model + remediation)

This plan owns: the GitHub-side target state + provisioning procedure above.
It does not own: card↔issue mapping rules, conflict resolution, the 18 cardless
issues (including the 3 P0s and 9 legacy 2023 issues — their labeling/
milestoning is remediation, not configuration), the 15-minute cron, or the
first `ghsync push`. The shared contract is the §3.1 board shape + the
`config.toml` values, which t_1cc489c3's remediation plan must consume as-is.
