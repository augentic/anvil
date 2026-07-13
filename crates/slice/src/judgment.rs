//! The slice loop's judgment leg: synthesis over the extracted
//! Evidence set. The schema-gated kernel (bounded repair loop, request
//! assembly, error mapping) lives in [`project::judgment`]; this module
//! carries the synthesize leg plus the embedded prompt corpus it
//! cites.

pub mod prose;
pub mod synthesize;

pub(crate) use project::judgment::{render_json, schema_gated};
