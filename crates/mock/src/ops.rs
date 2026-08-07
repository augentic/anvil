//! Canonical operations-trait implementors over the behaviour core.
//!
//! Each unit type binds one catalog identity onto the shared
//! [`crate::behaviour`] core: behaviour still keys off the routed
//! `ctx.adapter_id`, so one core serves every mock name and every
//! failure profile fails through the trait surface (no provider
//! hooks). The impls stay on the SDK seam DTOs end to end.

use adapter::registry::Doc;
use adapter::seam::{
    BuildContext, Context, Error, Evidence, Input, Lead, MergePhase, Report, SourceMetadata,
    TargetMetadata, Workspace,
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

            async fn survey<P: Model>(_model: &P, ctx: &Context<'_>) -> Result<Vec<Lead>, Error> {
                behaviour::survey(ctx.adapter_id)
            }

            async fn extract<P: Model>(
                _model: &P, ctx: &Context<'_>, lead: &Lead,
            ) -> Result<Evidence, Error> {
                behaviour::extract(ctx.adapter_id, lead)
            }
        }
    };
}

macro_rules! mock_target {
    ($ty:ident, $name:literal) => {
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
            ) -> Result<Report, Error> {
                // Artifact writes land in the private workspace; fail-
                // build markers are control-plane on the project tree
                // (RFC-86 D27: build prepares from a recorded pin, so
                // ambient markers must not require re-freeze).
                behaviour::build(
                    workspace.root_path(),
                    ctx.project_root,
                    ctx.adapter_id,
                    slice,
                    inputs,
                )
            }

            async fn merge<P: Model>(
                _model: &P, ctx: &Context<'_>, slice: &str, phase: MergePhase,
                _workspace: &Workspace,
            ) -> Result<Report, Error> {
                // Gate markers are test control-plane written into the
                // project tree after the build, so they are read through
                // the `"."` preopen — the read-only result view carries
                // only what the build captured.
                behaviour::merge(ctx.project_root, ctx.adapter_id, slice, phase)
            }
        }
    };
}

/// The default mock identity — both axes, like the WASM guest.
#[derive(Clone, Copy, Debug)]
pub struct Adapter;

mock_source!(Adapter, "mock");
mock_target!(Adapter, "mock");

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
