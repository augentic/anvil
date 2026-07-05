//! Slice `metadata.yaml`, lifecycle, and naming.
//!
//! Verb-level filesystem operations live in [`actions`].

pub mod actions;
pub mod build;
pub mod lifecycle;
pub mod metadata;
pub mod model;
pub mod outcome;
pub mod provenance;
pub mod synthesis;
pub mod validate;

pub use actions::{CreateIfExists, Created, Overlap};
pub use build::assemble::build_request;
pub use build::native_hook::run_native_build_hook;
pub use build::wire::{
    BuildArtifacts, BuildInputs, BuildOutput, BuildReport, BuildRequest, BuildStatus, UiSurface,
    enforce_report_no_blocking_on_success, enforce_report_outputs_exist,
    evaluate_ui_surface_coherence,
};
pub use lifecycle::LifecycleStatus;
pub use metadata::{Outcome, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec};
pub use model::SliceModel;
pub use outcome::Kind as OutcomeKind;
pub use synthesis::authority::{Agreement, ClaimRef, Resolution, resolve};
pub use synthesis::baseline::{BaselineIndex, DomainBaseline, DomainKind};
pub use synthesis::evidence::{
    KernelEvidence, evidence_path, read_evidence_index, read_source_inputs,
};
pub use synthesis::persist::{failure_reason as synthesize_failure_reason, persist_synthesized};
pub use synthesis::project::{ProjectionHeader, project};
pub use synthesis::render::{
    ExpectedRequirement, RenderedSpec, expected_provenance_lines, render_spec_files,
};
pub use synthesis::wire::{
    BaselineDomainDetail, SynthesisArtifacts, SynthesisInputs, SynthesisResponse,
    SynthesisSourceInput, SynthesisSpec, build_synthesis_inputs,
};

pub use crate::adapter::TargetOperation;
