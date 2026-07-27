//! Slice-loop guest orchestrators: one entry point per phase, each
//! dispatching across the [`project::seam`] capability traits and
//! owning its validate-before-visible tail.
//!
//! Extract persists schema-gated Evidence, build runs the finalize
//! tail (report schema gate, `enforce_report_*`, the `built`
//! transition, the `slice.build.*` bracket), and merge dispatches the
//! target's phased merge gates (preflight / postflight) around the
//! deterministic core commit. `emery source extract`, `emery slice
//! build`, and `emery slice merge run` route here through the guest;
//! the change loop (`emery plan execute`) drives the same entry
//! points per plan entry.
//!
//! Time is injected: every orchestrator takes the caller's `now`
//! (`docs/standards/architecture.md` §"Time injection"); library code
//! never reads the clock.

mod extract;
mod merge;
mod refine;
mod synthesize;
mod target;

pub use project::seam::Capabilities;
pub(crate) use project::seam::{seam_failure, target_id};

pub use self::extract::{ExtractOutcome, extract};
pub use self::merge::merge;
pub use self::refine::{RefineOutcome, refine, refine_breakout};
pub use self::synthesize::synthesize;
pub use self::target::build;
