//! The slice loop: synthesis, validation, provenance, the build-request
//! assembler, and the `specify slice *` operations. The slice data
//! model (`metadata.yaml`, lifecycle, outcome) lives in
//! [`project::slice`]; verb-level filesystem operations live in the
//! private actions module.

pub(crate) mod actions;
pub(crate) mod build;
pub mod handlers;
pub(crate) mod model;
pub(crate) mod provenance;
pub(crate) mod synthesis;
pub(crate) mod validate;

pub use project::seam::wire::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus, UiSurface};
pub use project::slice::LifecycleStatus;
pub(crate) use project::seam::wire::BuildRequest;
pub(crate) use project::slice::{
    Outcome, OutcomeKind, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec,
};

pub(crate) use actions::CreateIfExists;
pub(crate) use build::assemble::build_request;
pub(crate) use model::SliceModel;
pub(crate) use synthesis::baseline::BaselineIndex;
pub(crate) use synthesis::evidence::{read_evidence_index, read_source_inputs};
pub(crate) use synthesis::persist::{
    failure_reason as synthesize_failure_reason, persist_synthesized,
};
pub(crate) use synthesis::project::{ProjectionHeader, project};
pub(crate) use synthesis::render::provenance_lines;
pub(crate) use synthesis::wire::{
    DomainDetail, SourceInput, SynthesisInputs, SynthesisResponse, inputs,
};
