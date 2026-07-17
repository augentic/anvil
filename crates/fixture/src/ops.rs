//! Canonical operations-trait implementors over the behaviour core.
//!
//! Each unit type binds one catalog identity onto the shared
//! [`crate::behaviour`] core: behaviour still keys off the routed
//! `ctx.adapter_id`, so one core serves every fixture name and every
//! failure profile fails through the trait surface (no provider
//! hooks). The impls stay on the SDK seam DTOs end to end.

use adapter::registry::Doc;
use adapter::seam::{
    Context, Error, Evidence, Input, Lead, MergePhase, Report, SourceMetadata, TargetMetadata,
    WorkingTree,
};
use adapter::{Source, Target};
use omnia_guest::Model;

use crate::behaviour;

/// The fixture's single embedded reference document.
pub const DOCS: &[Doc] = &[Doc {
    path: "reference.md",
    body: "# Adapter Reference\n\nThe harness adapter serves both axes from one component: \
           deterministic survey/extract data on the source interface and guidance/build/merge \
           on the target interface.\n",
}];

macro_rules! fixture_source {
    ($ty:ident, $name:literal) => {
        impl Source for $ty {
            const NAME: &'static str = $name;

            fn metadata() -> SourceMetadata {
                SourceMetadata { specify_floor: None }
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

macro_rules! fixture_target {
    ($ty:ident, $name:literal) => {
        impl Target for $ty {
            const NAME: &'static str = $name;

            fn metadata() -> TargetMetadata {
                TargetMetadata {
                    specify_floor: None,
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
                _model: &P, ctx: &Context<'_>, slice: &str, inputs: &[Input], tree: &WorkingTree,
            ) -> Result<Report, Error> {
                let root = ctx.tree_root(tree);
                behaviour::build(&root, ctx.adapter_id, slice, inputs)
            }

            async fn merge<P: Model>(
                _model: &P, ctx: &Context<'_>, slice: &str, phase: MergePhase, tree: &WorkingTree,
            ) -> Result<Report, Error> {
                let root = ctx.tree_root(tree);
                behaviour::merge(&root, ctx.adapter_id, slice, phase)
            }
        }
    };
}

/// The default fixture identity — both axes, like the WASM guest.
#[derive(Clone, Copy, Debug)]
pub struct Adapter;

fixture_source!(Adapter, "fixture");
fixture_target!(Adapter, "fixture");

/// The documentation half of the adversarial source pair.
#[derive(Clone, Copy, Debug)]
pub struct Docs;

fixture_source!(Docs, "fixture-docs");

/// The behaviour (code) half of the adversarial source pair.
#[derive(Clone, Copy, Debug)]
pub struct Code;

fixture_source!(Code, "fixture-code");

/// A source whose `survey` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailSurvey;

fixture_source!(FailSurvey, "fixture-fail-survey");

/// A source whose `extract` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailExtract;

fixture_source!(FailExtract, "fixture-fail-extract");

/// A target whose `guidance` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailGuidance;

fixture_target!(FailGuidance, "fixture-fail-guidance");

/// A target whose `build` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailBuild;

fixture_target!(FailBuild, "fixture-fail-build");

/// A target whose `merge` gates fail with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailMerge;

fixture_target!(FailMerge, "fixture-fail-merge");

/// A target whose `build` reports success but never writes its
/// declared output — for the outputs-exist gate.
#[derive(Clone, Copy, Debug)]
pub struct MissingOutput;

fixture_target!(MissingOutput, "fixture-missing-output");
