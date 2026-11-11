//! ghsync keeps the Foodshareclub GitHub org in step with the Hermes
//! Kanban board `foodshare`.
//!
//! Kanban is the single source of state. GitHub has exactly one role per
//! surface:
//!
//! - **Projection** — `push` writes the org Project v2 fields, milestone
//!   assignments, issue labels and status. Board moves by humans are
//!   display-only and are reverted on the next push.
//! - **Intake** — issues submitted through the intake form (auto-`intake`
//!   label) and discussions labeled `intake` become kanban cards.
//! - **Feed** — review verdicts and escalations are posted to the repo's
//!   `Handoffs` discussion category.
//! - **Evidence** — merged-PR/CI comments are posted on the linked issue.
//!
//! The two sides are linked statelessly: each linked issue carries a
//! `kanban:<board>:<task-id>` footer, each card carries a
//! `ghsync <owner>/<repo>#<n>` comment. Either side can re-derive the map
//! from the other at any time; the only file on disk is a small cursor
//! that records the last-written field values for change detection.

pub mod cli;
pub mod config;
