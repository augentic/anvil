//! Change-loop guest orchestrators: plan authoring and the drained
//! execute loop behind `emery plan author` / `emery plan execute` /
//! `emery source survey`. Time is injected; nothing reads the clock.

mod author;
mod epoch;
mod execute;
mod gap_gate;
mod routing;
mod survey;

pub use project::seam::Capabilities;

pub use self::author::{AuthorOutcome, author};
pub use self::epoch::WaiveSelector;
pub use self::execute::{ExecuteOutcome, execute};
pub use self::gap_gate::enforce_before_build;
pub use self::survey::{SurveyedSource, survey, survey_all};
