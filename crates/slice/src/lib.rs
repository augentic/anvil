//! The Emery slice loop: refine / build / merge orchestration,
//! synthesis, validation, provenance, the delta-merge engine, and the
//! `emery slice *` operations. The slice data model (`metadata.yaml`,
//! lifecycle, outcome) and the deployment-neutral foundation live in
//! `project`; the change loop that drives this crate per plan entry
//! lives in `change`. See `docs/standards/architecture.md` for the
//! rationale.

pub(crate) mod actions;
pub mod answers;
pub mod base;
pub(crate) mod build;
pub(crate) mod design_system;
pub mod handlers;
pub(crate) mod judgment;
pub(crate) mod merge;
pub(crate) mod model;
pub mod orchestrate;
pub(crate) mod provenance;
pub mod source;
pub(crate) mod synthesis;
pub(crate) mod validate;

pub(crate) use actions::CreateIfExists;
pub use base::Base;
pub(crate) use build::assemble::build_request;
pub use model::SliceModel;
pub(crate) use project::seam::wire::BuildRequest;
pub use project::seam::wire::{BUILD_VERSION, BuildOutput, BuildReport, BuildStatus, UiSurface};
pub use project::slice::LifecycleStatus;
pub(crate) use project::slice::{
    Outcome, OutcomeKind, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec,
};
pub use synthesis::baseline::{BaselineIndex, DomainKind};
pub(crate) use synthesis::evidence::{read_evidence_index, read_source_inputs};
pub(crate) use synthesis::persist::{
    failure_reason as synthesize_failure_reason, persist_synthesized,
};
pub use synthesis::project::{ProjectionHeader, project};
pub(crate) use synthesis::render::provenance_lines;
pub(crate) use synthesis::wire::{
    DomainDetail, SourceInput, SynthesisInputs, SynthesisResponse, inputs,
};
