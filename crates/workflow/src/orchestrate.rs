//! Guest workflow orchestrators (RFC-61 Step 4, Milestone C).
//!
//! Each function collapses one of the old stack's two-phase CLI handoff
//! surfaces into a single call that dispatches across the
//! [`crate::seam`] capability traits and then runs the retired native
//! verb's validate-before-visible tail: survey fan-out feeds
//! `Discovery::merge_survey`, extract persists schema-gated Evidence,
//! build keeps the finalize tail (report schema gate, `enforce_report_*`,
//! the `built` transition, the `slice.build.*` bracket), and merge stays
//! deterministic-only per RFC-61 decision D2. Since Step 5 Milestone S4
//! these are the *only* code paths — the native two-phase envelope verbs
//! were deleted and `specify source survey/extract`, `specify slice
//! build`, and `specify slice merge run` route here through the guest.
//!
//! Three deliberate drops from the native build surface, per the step-3
//! precedent: no `prepare.argv` extension hook (targets own their
//! prelude in-guest) and no `host_prereq` / `finalize_verify` shell
//! hooks (verification moves agent-side into target prompts). The
//! guest merge additionally skips the workspace-clone git commit leg
//! with an explicit `slice.merge.commit-skipped` journal event —
//! lifecycle authority is `.specify/` state, so `done` still stamps.
//!
//! Time is injected: every orchestrator takes the caller's `now`
//! (architecture.md §"Time injection"); library code never reads the
//! clock.

mod author;
mod execute;
mod merge;
mod refine;
mod source;
mod synthesize;
mod target;

use specify_error::Error;

pub use self::author::{AuthorOutcome, author};
pub use self::execute::{ExecuteOutcome, GuestMarker, PhaseRun, execute};
pub use self::merge::{MergeOutcome, merge};
pub use self::refine::{RefineOutcome, refine, refine_breakout};
pub use self::source::{ExtractOutcome, SurveyedSource, extract, survey, survey_all};
pub use self::synthesize::{SynthesizeRequest, synthesize};
pub use self::target::{BuildOutcome, build};
use crate::seam;

/// Map a seam dispatch failure onto the wire contract.
///
/// `operation` is the seam method (`survey`, `extract`, `guidance`,
/// `build`); `id` is the routed adapter id (e.g. `source:typescript`).
fn seam_failure(operation: &'static str, id: &str, err: &seam::Error) -> Error {
    Error::Diag {
        code: "seam-dispatch-failed",
        detail: format!("seam `{operation}` dispatch to `{id}` failed: {err}"),
    }
}

/// The plan-bound adapter id routing a source dispatch
/// (`source:<adapter>`).
fn source_adapter_id(adapter: &str) -> String {
    format!("source:{adapter}")
}

/// The plan-bound adapter id routing a target dispatch
/// (`target:<name>`).
fn target_adapter_id(name: &str) -> String {
    format!("target:{name}")
}
