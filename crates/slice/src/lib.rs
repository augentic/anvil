//! The Emery slice loop: refine / build / merge orchestration,
//! synthesis, validation, provenance, the delta-merge engine, and the
//! `emery slice *` operations.

pub(crate) mod actions;
pub mod answers;
pub mod build;
pub mod debt;
pub(crate) mod design_system;
pub mod handlers;
pub(crate) mod judgment;
pub(crate) mod merge;
pub(crate) mod model;
pub mod orchestrate;
pub(crate) mod provenance;
pub mod refinement;
pub mod shelf;
pub mod source;
pub(crate) mod synthesis;
pub(crate) mod validate;

pub(crate) use actions::CreateIfExists;
pub use actions::discard;
pub(crate) use build::assemble::build_request;
pub use model::SliceModel;
pub(crate) use project::seam::wire::BuildRequest;
pub use project::seam::wire::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus, UiSurface};
pub use project::slice::LifecycleStatus;
pub(crate) use project::slice::{Outcome, OutcomeKind, SliceMetadata, SpecKind, TouchedSpec};
pub use synthesis::baseline::{BaselineIndex, DomainKind};
pub(crate) use synthesis::evidence::{read_evidence_index, read_source_inputs};
pub(crate) use synthesis::persist::{
    failure_reason as synthesize_failure_reason, persist_synthesized,
};
pub use synthesis::project::{ProjectionHeader, project};
pub(crate) use synthesis::render::provenance_lines;
pub(crate) use synthesis::wire::{
    DependencyContext, DomainDetail, SourceInput, SynthesisInputs, SynthesisResponse, inputs,
};
pub use validate::dispositions_drifted;
