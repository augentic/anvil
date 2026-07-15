//! The change loop's judgment leg: plan-time lead reconciliation. The
//! schema-gated kernel (bounded repair loop, request assembly, error
//! mapping) lives in [`project::judgment`]; this module carries the
//! propose leg plus the embedded prompt corpus it cites.

pub mod propose;
pub mod prose;
