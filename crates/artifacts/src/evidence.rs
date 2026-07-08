//! source-adapter Evidence shapes.
//!
//! Per-source `extract` output, persisted at
//! `.specify/slices/<slice>/evidence/<source>.yaml` and validated
//! against `schemas/evidence.schema.json`. The module collects the
//! closed `AuthorityClass` / `ClaimKind` enums shared with the schema
//! and `plan.yaml`'s per-slice `authority-override` map.

pub mod authority;
pub mod claim;

pub use authority::{AuthorityClass, ClaimKind};
pub use claim::ExampleClaim;
