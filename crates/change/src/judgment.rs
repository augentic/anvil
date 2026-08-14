//! The change loop's judgment legs: partition, boundary review, and
//! plan-time lead reconciliation. Schema-gated kernels live in
//! [`project::judgment`]; this crate owns the prompt corpus.

pub mod partition;
pub mod propose;
pub mod prose;
pub mod review;
