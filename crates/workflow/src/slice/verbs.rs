//! The `specify slice *` verb family (plus `archive prune`, whose
//! kernel lives in [`super::actions::prune`]). `slice build` / `slice
//! refine` / `slice merge run` are orchestration verbs and live in
//! [`crate::orchestrate::verbs`].

mod lifecycle;
mod merge;
mod model;
mod provenance;
mod prune;
mod task;
mod touched;
mod validate;

/// Re-exported so the verb submodules and orchestration callers keep
/// one import path to the merge synthesiser's class table.
pub use crate::merge::artifact_classes;

pub use self::lifecycle::{
    Create, CreateInput, Drop, DropBody, DropInput, Transition, TransitionBody, TransitionInput,
};
pub use self::merge::{
    ConflictCheck, ConflictCheckBody, ConflictCheckInput, Preview, PreviewBody, PreviewInput,
};
pub use self::model::{ModelShow, ModelShowInput};
pub use self::provenance::{Provenance, ProvenanceInput};
pub use self::prune::{Prune, PruneBody, PruneInput};
pub use self::task::{
    MarkBody, ProgressBody, TaskMark, TaskMarkInput, TaskProgress, TaskProgressInput,
};
pub use self::touched::{
    Overlap, OverlapBody, OverlapInput, SpecsBody, TouchedSpecs, TouchedSpecsInput,
};
pub use self::validate::{Validate, ValidateInput};
