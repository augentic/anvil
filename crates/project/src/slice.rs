//! The slice data model shared across the stack.
//!
//! Carries `metadata.yaml`, projected lifecycle labels, and the
//! phase-outcome record. The slice loop itself (refine / build / merge
//! orchestration) lives in the `slice` crate; this module carries only
//! the types every layer reads — plan execution predicates resolve
//! slice progress from artifacts and facts (RFC-86 D2).

pub mod lifecycle;
pub mod metadata;
pub mod outcome;

pub use lifecycle::LifecycleStatus;
pub use metadata::{
    Outcome, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec, slice_not_found,
};
pub use outcome::Kind as OutcomeKind;
