//! The slice data model shared across the stack.
//!
//! Carries `metadata.yaml`, the lifecycle state machine, and the
//! phase-outcome record. The slice loop itself (refine / build / merge
//! orchestration) lives in the `slice` crate; this module carries only
//! the types every layer reads — the plan execution predicates resolve
//! slice state and init-time context generation fingerprints it.

pub mod lifecycle;
pub mod metadata;
pub mod outcome;

pub use lifecycle::LifecycleStatus;
pub use metadata::{
    Outcome, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec, slice_not_found,
};
pub use outcome::Kind as OutcomeKind;
