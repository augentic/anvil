//! Change-loop guest orchestrators: plan authoring (survey fan-out →
//! reconciliation judgment → persist → Gate 1 prose) and the drained
//! execute loop that dispatches the per-slice refine / build / merge
//! phases through the `slice` crate. `specify plan author`, `specify
//! plan execute`, and `specify source survey` route here through the
//! guest.
//!
//! Time is injected: every orchestrator takes the caller's `now`
//! (`docs/standards/architecture.md` §"Time injection"); library code
//! never reads the clock.

mod author;
mod execute;
mod routing;
mod survey;

pub use project::seam::Capabilities;

pub use self::author::author;
pub use self::execute::{ExecuteOutcome, execute};
pub use self::survey::{SurveyedSource, survey, survey_all};
