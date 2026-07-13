//! Target build envelope kernel.
//!
//! Mirrors [`crate::synthesis`]: the pure, IO-free domain pieces
//! the `specify slice build` verb composes. The closed-shape build
//! request/report DTOs live in [`project::seam::wire`]; [`assemble`]
//! assembles a request from the bound target adapter's declared inputs
//! against the slice tree. Schema validation of the raw envelopes lives
//! in [`project::schema_gate`], beside the other workflow-aware
//! validators.

pub mod assemble;
