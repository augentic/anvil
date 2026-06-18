//! Target build envelope kernel.
//!
//! Mirrors [`crate::slice::synthesis`]: the pure, IO-free domain pieces
//! the `specify slice build` verb composes. [`wire`] holds the
//! closed-shape build request/report DTOs (round-tripping
//! `schemas/target/build-request.schema.json` and
//! `schemas/target/build-report.schema.json`) plus the
//! success-with-blocking gate; [`assemble`] assembles a request from
//! the bound target adapter's declared inputs against the slice tree.
//! Schema validation of the raw envelopes lives in
//! [`crate::schema`], beside the other workflow-aware validators.

pub mod assemble;
pub mod materialize_scope;
pub mod wire;
