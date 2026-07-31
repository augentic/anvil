//! The `emery slice *` command operations, plus `archive prune`.
//!
//! The prune kernel lives in the private actions module; the refine /
//! build / merge operations drive their internal kernels (slice
//! create, lifecycle transitions, touched-spec scans, and task writes
//! have no verb surface — the orchestrations own them).

mod build;
mod lifecycle;
mod list;
mod merge;
mod model;
mod provenance;
mod prune;
mod refine;
mod validate;

pub use self::build::{Build, BuildBody, BuildInput};
pub use self::lifecycle::{Drop, DropBody, DropInput};
pub use self::list::{List, ListBody, ListEntry, ListInput};
pub use self::merge::{
    ConflictCheckBody, MergeBody, MergeRun, MergeRunBody, MergeRunInput, PreviewBody,
};
pub use self::model::{ModelShow, ModelShowInput};
pub use self::provenance::{Provenance, ProvenanceInput};
pub use self::prune::{Prune, PruneBody, PruneInput};
pub use self::refine::{Refine, RefineBody, RefineExtract, RefineInput, RefineTags};
pub use self::validate::{Validate, ValidateInput};
