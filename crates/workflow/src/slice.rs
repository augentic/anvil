//! Slice `metadata.yaml`, lifecycle, and naming.
//!
//! Verb-level filesystem operations live in [`actions`].

pub(crate) mod actions;
pub(crate) mod build;
pub mod handlers;
mod lifecycle;
pub(crate) mod metadata;
pub(crate) mod model;
pub(crate) mod outcome;
pub(crate) mod provenance;
pub(crate) mod synthesis;
pub(crate) mod validate;

pub use actions::CreateIfExists;
pub(crate) use actions::{Created, Overlap};
pub(crate) use build::assemble::build_request;
pub(crate) use build::wire::BuildRequest;
pub use build::wire::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus, UiSurface};
pub use lifecycle::LifecycleStatus;
pub(crate) use metadata::{Outcome, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec};
pub(crate) use model::SliceModel;
pub(crate) use outcome::Kind as OutcomeKind;
pub(crate) use synthesis::baseline::BaselineIndex;
pub(crate) use synthesis::evidence::{read_evidence_index, read_source_inputs};
pub(crate) use synthesis::persist::{
    failure_reason as synthesize_failure_reason, persist_synthesized,
};
pub(crate) use synthesis::project::{ProjectionHeader, project};
pub(crate) use synthesis::render::expected_provenance_lines;
pub(crate) use synthesis::wire::{
    BaselineDomainDetail, SynthesisInputs, SynthesisResponse, SynthesisSourceInput,
    build_synthesis_inputs,
};
