//! The slice loop's judgment leg: synthesis over extracted Evidence.
//!
//! The schema-gated kernel lives in [`project::judgment`]; this module
//! carries the synthesize leg and its embedded prompt corpus.

pub mod prose;
pub mod synthesize;

pub use project::judgment::{render_json, repaired};
