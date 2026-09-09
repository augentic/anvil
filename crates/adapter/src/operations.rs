//! The adapter contract
//!
//! [`SourceAdapter`] is what an adapter implements: its identity, its
//! resolve-time metadata, the reference documents it embeds, and the
//! `extract` operation that reads a source and returns evidence.
//!
//! Keeping the trait separate from the wasm export lets an adapter be
//! exercised natively against a scripted model, with the component wiring
//! added only at the guest boundary.

use std::future::Future;

use emery_prose::registry::Doc;
use omnia_guest::{Error, Model};

use crate::types::{Context, Evidence, SourceInput, SourceMetadata};

/// Contract implemented by source adapters.
///
/// Generic over [`Model`] for native test doubles and the wasm host model;
/// deliberately not object-safe.
pub trait SourceAdapter {
    /// Compile-time `name@version` identity.
    const IDENTITY: &str;

    /// Resolve-time metadata; by default the SDK's own version is the
    /// exact `emery` pin.
    #[must_use]
    fn metadata() -> SourceMetadata {
        SourceMetadata {
            emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }
    }

    /// Embedded prose registry.
    fn docs() -> &'static [Doc];

    /// Extract the source's claim set.
    ///
    /// Refuse unusable input with `BadRequest`; the engine reports any other
    /// class as an adapter failure.
    fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;
}
