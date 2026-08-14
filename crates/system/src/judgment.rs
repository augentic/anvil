//! The definition loop's judgment legs: correlation over the complete
//! extracted Evidence set, and the initial-plan proposal. The
//! schema-gated kernel lives in [`project::judgment`].

pub mod correlate;
pub mod propose;
pub mod prose;

pub use project::judgment::{render_json, repaired};
