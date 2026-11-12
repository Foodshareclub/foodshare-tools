# ghsync

`ghsync` keeps the Foodshareclub GitHub org in step with the Hermes
Kanban board `foodshare`. The board is the single source of state;
GitHub is a projection, an intake channel, a feed, and an evidence
channel — one role per surface, never a second state.

## Surface map

| GitHub surface            | Role      | Direction    | Behaviour                                                                                     |
| ------------------------- | --------- | ------------ | --------------------------------------------------------------------------------------------- |
| Issues + labels           | Projection | kanban → GH  | One issue per card, `ghsync` + `ghsync-<repo>` labels, `kanban:` footer, milestone, open/closed state. |
| Intake form (`agent-intake.yml`) | Intake   | GH → kanban  | A submitted form is an `intake`-labeled issue; `intake` turns it into exactly one card.        |
| Discussions (labeled)     | Intake    | GH → kanban  | A discussion labeled `intake` becomes one card; the label is removed.                          |
| Org Project v2 board      | Projection | kanban → GH  | Status / Assignee / Phase fields per linked issue, written only when the value changes.        |
| Feed discussions          | Feed      | kanban → GH  | Review verdicts and escalations post to the repo's feed category (`feed_category`, default `General`), one discussion per card. |
| Merged-PR comments        | Evidence  | GH → GH      | A merged PR referenced by a card leaves exactly one `PR #n merged - CI: …` comment on the linked issue. |

## Commands

```bash
ghsync setup                      # verify token; provision labels, project, fields, views (idempotent)
ghsync push [--dry-run]           # projection + PR evidence + discussion feed
ghsync intake [--dry-run]         # form issues + labeled discussions → cards
ghsync status [--json]            # read-only drift table across every surface
```

`setup` discovers the project id and single-select option ids and
writes them back into the config. It also verifies that each
discussions-enabled repo's configured `feed_category` exists among the
repo's pre-existing categories — discussion categories cannot be
created via the GitHub API — and fails loudly with the available list
when it does not. `push` runs the projection, the PR evidence, and the
feed in that order; a dry run plans without writing and without moving
the cursor.

## Conventions

- **The intake form is the intake trigger.** The form in
  `.github/ISSUE_TEMPLATE/agent-intake.yml` submits an issue with the
  `intake` label; nothing else starts intake. Form fields are
  `**label:**` lines (Phase, Agent profile, Acceptance); a missing
  Phase means Ad-hoc, a missing profile means `default_assignee`.
- **`Repo: <name>` line** in a card body routes the card to a repo;
  absent it goes to `foodshare-tools`. An unknown name is drift, not
  an error.
- **Links are stateless comment grammar.** Each linked issue carries a
  `kanban:<board>:<task-id>` footer in its body; each card carries a
  `ghsync <owner>/<repo>#<n>` comment (discussions use `ghsyncd
  <owner>/<repo>#<n>`). Either side rebuilds the other at any time;
  the only file on disk is the cursor.
- **Project fields are display-only.** A human move on the GitHub
  board is display-only and is reverted on the next push. Change state
  in kanban, or use the issue close/reopen gestures below. `status`
  reports a hand-moved field as `field-mismatch` until the next push
  corrects it.
- **The cursor is the change detector.** A project field is written
  only when the computed value differs from the cursor's last written
  value; steady state costs zero GraphQL reads and writes.

## What is NOT mirrored

- **Human edits to issue bodies/labels** are not read back as state —
  labels are re-projected and the footer repaired on the next push.
  The two gestures below are the only human issue actions ghsync
  honors.
- **Discussion state** (comments, categories, labels other than
  `intake`) is not mirrored either way; ghsync only creates/appends in
  the repo's feed category and reads the `intake` label.
- **PR actions** (merge, approve, review) are never performed by
  ghsync. A merge only produces an evidence comment; it never
  completes or blocks the card.

## The two human gestures that mean something

| Human action on the issue | Kanban effect                                      |
| ------------------------- | -------------------------------------------------- |
| Close the issue           | Card blocked (reason: `issue closed on GitHub (ghsync)`). |
| Reopen the issue          | Card unblocked (reason: `issue reopened on GitHub (ghsync)`). |

## Verdicts and the feed

A card comment prefixed `sync:github ` carries a review verdict
(`sync:github approve`, `sync:github request-changes`); a block
reason prefixed `escalation:` is an escalation. `push` mirrors each
to a discussion in the repo's feed category (config `feed_category`,
default `General`) titled `[<verdict>] <card title>`, and appends to
the same discussion on later runs — never a second one. The card
reference line in the discussion body
(`Card: kanban:<board>:<id>`) is what re-derives the link.

## Cadence and operations

- The sync runs every 15 minutes: `ghsync push && ghsync intake`,
  followed by `ghsync status` in the health check. The cron job is
  created by the operator after the first live `setup`.
- `status` is the health check: zero drift rows is the steady state.
  Each drift flag names the surface that moved: `unlinked`, `orphan`,
  `label-mismatch`, `field-mismatch`, `closed-but-not-done`,
  `footer-missing`, `pr-evidence-missing`, `feed-due`.
- **Bot-account upgrade path:** until a dedicated bot account exists,
  the PAT is a personal fine-grained token and ghsync-authored
  content is attributed to that person. The upgrade is a config
  change (new token) — no behaviour changes, and no migration of
  linked issues is needed because links are comment grammar, not
  authorship.
- **Per-repo opt-outs:** a repo with `discussions_enabled = false`
  (the default until `setup` records otherwise) skips the feed and
  the discussion intake for that repo, and the skip is logged.
  `discussion_labels_enabled = false` skips discussion intake only.
  `feed_category` (default `General`) names the pre-existing category
  the feed posts into; `setup` verifies it and names the available
  categories when it is missing.

## Token

Fine-grained PAT, org `Foodshareclub`, all repositories: Issues
read/write, Projects read/write, Discussions read/write, Pull
requests read; 90-day expiry. It lives in the config file only.
