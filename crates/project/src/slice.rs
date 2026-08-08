//! The slice data model shared across the stack.
//!
//! Carries `metadata.yaml`, projected lifecycle labels, and the
//! phase-outcome record; the slice loop itself lives in the `slice` crate.

pub mod lifecycle;
pub mod metadata;
pub mod outcome;

pub use lifecycle::LifecycleStatus;
pub use metadata::{
    Outcome, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec, slice_not_found,
};
pub use outcome::Kind as OutcomeKind;
