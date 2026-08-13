//! Change-loop guest orchestrators.
//!
//! Plan authoring, the serial refinement drain, and the drained
//! execute loop. Time is injected; nothing reads the clock.

mod author;
mod decompose;
mod epoch;
mod escalate;
mod execute;
mod gap_gate;
mod refine;
mod survey;

pub use project::seam::Capabilities;

pub use self::author::{AuthorOutcome, author};
pub use self::execute::{ExecuteOutcome, execute};
pub use self::gap_gate::enforce_before_build;
pub use self::refine::{RefineOutcome, refine};
pub use self::survey::{SurveyedSource, survey, survey_all};
