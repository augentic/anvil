//! Guest workflow orchestrators (RFC-61 Step 4, Milestone C).
//!
//! Each function collapses one of today's two-phase CLI handoff
//! surfaces into a single call that dispatches across the
//! [`crate::seam`] capability traits and then runs the native verb's
//! validate-before-visible tail: survey fan-out feeds
//! `Discovery::merge_survey`, extract persists schema-gated Evidence,
//! build keeps the finalize tail (report schema gate, `enforce_report_*`,
//! the `built` transition, the `slice.build.*` bracket), and merge stays
//! deterministic-only per RFC-61 decision D2. The native envelope verbs
//! (`specify source survey/extract`, `specify slice build --phase`,
//! `specify slice merge`) are untouched — these are new code paths the
//! Milestone D guest shim will drive.
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

mod execute;
mod merge;
mod refine;
mod source;
mod synthesize;
mod target;

use specify_error::Error;

pub use self::execute::{ExecuteOutcome, GuestMarker, PhaseRun, execute};
pub use self::merge::{MergeOutcome, merge};
pub use self::refine::{RefineOutcome, refine};
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
