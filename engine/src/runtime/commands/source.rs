//! `specify source {resolve, preview, survey, extract}` — source
//! adapter operations.
//!
//! `resolve` shares the run-side dispatch with the target axis on the
//! unified `commands::resolve_plugin` helper (it is byte-identical to
//! the target-axis path apart from the `@version` peel). `preview` is
//! the `specify source preview` contract workflow-free source adapter
//! scaffolding verb. [`prep`] is the internal prep seam (adapter
//! resolution, brief directory, the four-root sandbox layout, and
//! `evidence/` scaffolding) shared by `preview` today and the
//! `survey` / `extract` runners.

pub mod cli;
pub mod extract;
pub mod op;
pub mod prep;
pub mod preview;
pub mod survey;
