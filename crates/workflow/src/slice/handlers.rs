//! The `specify slice *` command operations, plus `archive prune`.
//!
//! The prune kernel lives in [`super::actions::prune`]; the refine /
//! build / merge operations drive their [`crate::orchestrate`]
//! kernels.

mod build;
mod lifecycle;
mod merge;
mod model;
mod provenance;
mod prune;
mod refine;
mod task;
mod touched;
mod validate;

pub use self::build::{Build, BuildBody, BuildInput};
pub use self::lifecycle::{
    Create, CreateInput, Drop, DropBody, DropInput, Transition, TransitionBody, TransitionInput,
};
pub use self::merge::{
    ConflictCheck, ConflictCheckBody, ConflictCheckInput, MergeBody, MergeRun, MergeRunInput,
    Preview, PreviewBody, PreviewInput,
};
pub use self::model::{ModelShow, ModelShowInput};
pub use self::provenance::{Provenance, ProvenanceInput};
pub use self::prune::{Prune, PruneBody, PruneInput};
pub use self::refine::{Refine, RefineBody, RefineExtract, RefineInput};
pub use self::task::{
    MarkBody, ProgressBody, TaskMark, TaskMarkInput, TaskProgress, TaskProgressInput,
};
pub use self::touched::{
    Overlap, OverlapBody, OverlapInput, SpecsBody, TouchedSpecs, TouchedSpecsInput,
};
pub use self::validate::{Validate, ValidateInput};
