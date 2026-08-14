//! Canonical operations-trait implementors over the behaviour core.
//!
//! Each unit type binds one catalog identity onto [`crate::behaviour`];
//! behaviour keys off the routed `ctx.adapter_id`, so one core serves all.

use adapter::registry::Doc;
use adapter::seam::{
    BuildContext, Context, Error, Evidence, Input, MergePhase, PhaseFinding, PhaseReport,
    RepairOrigin, Report, SourceInput, SourceMetadata, SurveyResult, TargetMetadata, Workspace,
    WritableArtifact,
};
use adapter::{AdapterIdentity, Source, Target};
use omnia_guest::Model;

use crate::behaviour;

/// The mock's single embedded reference document.
pub const DOCS: &[Doc] = &[Doc {
    path: "reference.md",
    body: "# Adapter Reference\n\nThe mock adapter serves both axes from one component: \
           deterministic survey/extract data on the source interface and guidance/build/merge \
           on the target interface.\n",
}];

macro_rules! mock_source {
    ($ty:ident, $name:literal) => {
        impl Source for $ty {
            // Unpublished mock identity: a development placeholder
            // version, never a pin-matchable release.
            const IDENTITY: AdapterIdentity = AdapterIdentity {
                name: $name,
                version: "0.0.0",
            };

            fn metadata() -> SourceMetadata {
                SourceMetadata { emery_floor: None }
            }

            fn docs() -> &'static [Doc] {
                DOCS
            }

            async fn survey<P: Model>(
                _model: &P, ctx: &Context<'_>, input: &SourceInput,
            ) -> Result<SurveyResult, Error> {
                behaviour::survey(ctx.adapter_id, input)
            }

            async fn extract<P: Model>(
                _model: &P, ctx: &Context<'_>, input: &SourceInput,
            ) -> Result<Evidence, Error> {
                behaviour::extract(ctx.adapter_id, input)
            }
        }
    };
}

macro_rules! mock_target {
    ($ty:ident, $name:literal) => {
        mock_target!($ty, $name, Vec::new());
    };
    ($ty:ident, $name:literal, $writable:expr) => {
        impl Target for $ty {
            // Unpublished mock identity: a development placeholder
            // version, never a pin-matchable release.
            const IDENTITY: AdapterIdentity = AdapterIdentity {
                name: $name,
                version: "0.0.0",
            };

            fn metadata() -> TargetMetadata {
                TargetMetadata {
                    emery_floor: None,
                    inputs: Vec::new(),
                    platforms: None,
                    writable_artifacts: $writable,
                }
            }

            fn docs() -> &'static [Doc] {
                DOCS
            }

            async fn guidance<P: Model>(_model: &P, ctx: &Context<'_>) -> Result<String, Error> {
                behaviour::guidance(ctx.adapter_id)
            }

            async fn build<P: Model>(
                _model: &P, ctx: &Context<'_>, slice: &str, inputs: &[Input],
                _context: &BuildContext, workspace: &Workspace,
            ) -> Result<PhaseReport, Error> {
                // Artifact writes land in the private workspace and its
                // lent stage; control markers stay on the project tree
                // so they never require re-freezing the recorded pin.
                behaviour::build(workspace, ctx.project_root, ctx.adapter_id, slice, inputs)
            }

            async fn verify<P: Model>(
                _model: &P, ctx: &Context<'_>, workspace: &Workspace,
            ) -> Result<PhaseReport, Error> {
                behaviour::verify(workspace, ctx.project_root, ctx.adapter_id)
            }

            async fn repair<P: Model>(
                _model: &P, _ctx: &Context<'_>, _slice: &str, origin: RepairOrigin,
                _findings: &[PhaseFinding], _continuation: Option<&[u8]>, workspace: &Workspace,
            ) -> Result<PhaseReport, Error> {
                behaviour::repair(workspace, origin)
            }

            async fn review<P: Model>(
                _model: &P, ctx: &Context<'_>, _slice: &str, continuation: Option<&[u8]>,
                workspace: &Workspace,
            ) -> Result<PhaseReport, Error> {
                behaviour::review(workspace, ctx.project_root, continuation)
            }

            async fn merge<P: Model>(
                _model: &P, ctx: &Context<'_>, slice: &str, phase: MergePhase,
                _workspace: &Workspace,
            ) -> Result<Report, Error> {
                // Gate markers are written into the project tree after the
                // build, so they are read through the `"."` preopen — the
                // read-only result view carries only what the build captured.
                behaviour::merge(ctx.project_root, ctx.adapter_id, slice, phase)
            }
        }
    };
}

/// The default mock identity — both axes, like the WASM guest. The
/// target axis declares the `tasks.md` file grant so its builds may
/// write the lent artifact stage.
#[derive(Clone, Copy, Debug)]
pub struct Adapter;

mock_source!(Adapter, "mock");
mock_target!(Adapter, "mock", vec![WritableArtifact::file("tasks.md")]);

/// The documentation half of the adversarial source pair.
#[derive(Clone, Copy, Debug)]
pub struct Docs;

mock_source!(Docs, "mock-docs");

/// The behaviour (code) half of the adversarial source pair.
#[derive(Clone, Copy, Debug)]
pub struct Code;

mock_source!(Code, "mock-code");

/// A source whose `survey` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailSurvey;

mock_source!(FailSurvey, "mock-fail-survey");

/// A source whose `extract` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailExtract;

mock_source!(FailExtract, "mock-fail-extract");

/// A target whose `guidance` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailGuidance;

mock_target!(FailGuidance, "mock-fail-guidance");

/// A target whose `build` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailBuild;

mock_target!(FailBuild, "mock-fail-build");

/// A target whose `merge` gates fail with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailMerge;

mock_target!(FailMerge, "mock-fail-merge");

/// A target whose `build` reports success but never writes its
/// declared output — for the outputs-exist gate.
#[derive(Clone, Copy, Debug)]
pub struct MissingOutput;

mock_target!(MissingOutput, "mock-missing-output");

/// A target whose `verify` claims the reserved `tool` report source —
/// for the RFC-90 source gate.
#[derive(Clone, Copy, Debug)]
pub struct ToolSource;

mock_target!(ToolSource, "mock-tool-source");

/// A target whose `verify` declares non-empty outputs — only `build`
/// may declare outputs (an engine gate).
#[derive(Clone, Copy, Debug)]
pub struct VerifyOutputs;

mock_target!(VerifyOutputs, "mock-verify-outputs");

/// A target whose `verify` returns `not-applicable` while carrying a
/// blocking finding — for the non-applicable coherence gate.
#[derive(Clone, Copy, Debug)]
pub struct NaBlocking;

mock_target!(NaBlocking, "mock-na-blocking");

/// A target whose `build` returns a continuation one byte over the
/// engine's 1 MiB cap — for the continuation-size gate.
#[derive(Clone, Copy, Debug)]
pub struct OversizedContinuation;

mock_target!(OversizedContinuation, "mock-oversized-continuation");

/// A target whose `build` writes an undeclared staged artifact
/// (`undeclared.md`) despite holding only the `tasks.md` grant — for
/// the staged-artifact scope gate.
#[derive(Clone, Copy, Debug)]
pub struct StageEscape;

mock_target!(StageEscape, "mock-stage-escape", vec![WritableArtifact::file("tasks.md")]);

/// A target whose `verify` replaces the continuation — `verify` must
/// not mutate it (an engine gate).
#[derive(Clone, Copy, Debug)]
pub struct VerifyContinuation;

mock_target!(VerifyContinuation, "mock-verify-continuation");
