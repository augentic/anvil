//! Change-loop guest orchestrators.
//!
//! Plan authoring (survey fan-out → reconciliation judgment → persist
//! → Gate 1 prose) and the drained execute loop that dispatches the
//! per-slice refine / build / merge phases through the `slice` crate.
//! `emery plan author`, `emery plan execute`, and `emery source
//! survey` route here through the guest.
//!
//! Time is injected: every orchestrator takes the caller's `now`
//! (`docs/standards/architecture.md` §"Time injection"); library code
//! never reads the clock.

mod author;
mod execute;
mod routing;
mod survey;

pub use project::seam::Capabilities;

pub use self::author::{AuthorOutcome, author};
pub use self::execute::{ExecuteOutcome, execute};
pub use self::survey::{SurveyedSource, survey, survey_all};
