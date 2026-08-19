//! Canonical operations-trait implementors over the behaviour core.
//!
//! Each unit type binds one catalog identity onto [`crate::behaviour`];
//! behaviour keys off the routed `ctx.adapter_id`, so one core serves all.

use adapter::registry::Doc;
use adapter::seam::{Context, Error, Evidence, SourceInput, SourceMetadata};
use adapter::{AdapterIdentity, Source};
use omnia_guest::Model;

use crate::behaviour;

/// The mock's single embedded reference document.
pub const DOCS: &[Doc] = &[Doc {
    path: "reference.md",
    body: "# Adapter Reference\n\nThe mock source adapter serves deterministic extract data on \
           the source interface.\n",
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

            async fn extract<P: Model>(
                _model: &P, ctx: &Context<'_>, input: &SourceInput,
            ) -> Result<Evidence, Error> {
                behaviour::extract(ctx.adapter_id, input)
            }
        }
    };
}

/// The default mock identity.
#[derive(Clone, Copy, Debug)]
pub struct Adapter;

mock_source!(Adapter, "mock");

/// The documentation half of the adversarial source pair.
#[derive(Clone, Copy, Debug)]
pub struct Docs;

mock_source!(Docs, "mock-docs");

/// The behaviour (code) half of the adversarial source pair.
#[derive(Clone, Copy, Debug)]
pub struct Code;

mock_source!(Code, "mock-code");

/// The inline operator-intent source that outranks the pair.
#[derive(Clone, Copy, Debug)]
pub struct Intent;

mock_source!(Intent, "mock-intent");

/// A source whose `extract` fails with a typed internal error.
#[derive(Clone, Copy, Debug)]
pub struct FailExtract;

mock_source!(FailExtract, "mock-fail-extract");

/// A source violating A8: a requirement claim missing its required
/// `statement` extra, for the engine's fail-closed gate.
#[derive(Clone, Copy, Debug)]
pub struct MissingExtras;

mock_source!(MissingExtras, "mock-missing-extras");
