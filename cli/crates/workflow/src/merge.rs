//! Deterministic delta-merge engine. [`merge`] folds one delta into a
//! baseline; [`validate_baseline`] runs post-merge coherence checks;
//! [`slice::commit`] is the transactional multi-class merge + archive.

mod artifact_class;
pub mod clone_commit;
pub mod composition;
mod engine;
pub mod slice;
mod validate;

pub use artifact_class::{ArtifactClass, MergeStrategy};
pub use engine::{MergeOperation, MergeResult, merge};
pub use slice::{
    BaselineConflict, MergeCommit, MergePreviewEntry, OpaqueAction, OpaquePreviewEntry,
    PreviewResult, conflict_check,
};
pub use validate::validate_baseline;
