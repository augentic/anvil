//! Canonical scenario definitions, assertion taxonomy, and structured
//! run reports.
//!
//! This crate owns serialisation and validation only. Execution remains
//! with the native and WebAssembly harnesses that already own runtime
//! and model mechanics.

mod assertion;
/// Deterministic grading over captured workflow-step evidence.
pub mod grade;
mod model;
mod report;

pub use assertion::{AssertionId, AssertionKind, AssertionMetadata, assertion_registry};
pub use model::{
    ExpectedOutput, Fixture, GateTier, Grading, HardAssertion, Isolation, ModelBackend, OutputKind,
    Probe, Profile, Runtime, Scenario, ScenarioVersion, SemanticRubric, Setup, Stream,
    WorkflowStep, WorkflowStepKind,
};
pub use report::{
    AssertionResult, Outcome, RubricResult, RunMetadata, ScenarioReport, ScenarioReportVersion,
    TrialMetrics, TrialResult,
};
