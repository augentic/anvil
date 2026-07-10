//! Guest workflow orchestrators: one entry point per phase, each
//! dispatching across the [`crate::seam`] capability traits and owning
//! its validate-before-visible tail.
//!
//! Survey fan-out feeds `Discovery::merge_survey`, extract persists
//! schema-gated Evidence, build runs the finalize tail (report schema
//! gate, `enforce_report_*`, the `built` transition, the
//! `slice.build.*` bracket), and merge is deterministic-only. `specify
//! source survey/extract`, `specify slice build`, and `specify slice
//! merge run` route here through the guest.
//!
//! Time is injected: every orchestrator takes the caller's `now`
//! (architecture.md §"Time injection"); library code never reads the
//! clock.

mod author;
mod execute;
pub mod handlers;
mod merge;
mod refine;
mod source;
mod synthesize;
mod target;

use error::Error;

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
