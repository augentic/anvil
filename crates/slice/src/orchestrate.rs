//! Slice-loop guest orchestrators: one entry point per phase, each
//! dispatching across the [`project::seam`] capability traits and
//! owning its validate-before-visible tail.

mod extract;
mod merge;
mod refine;
mod synthesize;
mod target;

pub use project::seam::Capabilities;
pub(crate) use project::seam::{seam_failure, target_id};

pub use self::extract::{ExtractOutcome, extract};
pub use self::merge::{MergeOutcome, merge};
pub use self::refine::{RefineOutcome, refine};
pub use self::synthesize::synthesize;
pub use self::target::{BuildOutcome, build, open_wave_group};
