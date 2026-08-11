//! The slice data model shared across the stack: `metadata.yaml`,
//! projected lifecycle labels, the phase-outcome record, and the
//! requirement-body digest; the slice loop lives in the `slice` crate.

pub mod lifecycle;
pub mod metadata;
pub mod outcome;
pub mod requirement;

pub use lifecycle::{LifecycleStatus, has_spec_artifacts};
pub use metadata::{
    Outcome, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec, slice_not_found,
};
pub use outcome::Kind as OutcomeKind;
pub use requirement::RequirementBody;
