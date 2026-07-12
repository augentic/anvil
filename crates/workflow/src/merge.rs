//! Deterministic delta-merge engine. [`merge`] folds one delta into a
//! baseline; [`validate_baseline`] runs post-merge coherence checks;
//! [`slice::commit`] is the transactional multi-class merge + archive.

mod artifact_class;
mod composition;
mod engine;
pub mod slice;
mod validate;

pub use artifact_class::{MergeStrategy, artifact_classes};
pub use engine::MergeOperation;
pub use slice::{
    BaselineConflict, MergeCommit, OpaqueAction, PreviewEntry, conflict_check, summarise_operations,
};
