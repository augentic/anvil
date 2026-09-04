//! Export-side source adapter contract.

use std::future::Future;

use emery_prose::registry::Doc;
use omnia_guest::Model;

use crate::types::{Context, Error, Evidence, SourceInput, SourceMetadata};

/// Contract implemented by source adapters.
///
/// Generic over [`Model`] for native test doubles and the wasm host model;
/// deliberately not object-safe.
pub trait SourceAdapter {
    /// Compile-time `name@version` identity.
    const IDENTITY: &str;

    /// Resolve-time metadata.
    fn metadata() -> SourceMetadata;

    /// Embedded prose registry.
    fn docs() -> &'static [Doc];

    /// Extract the source's claim set.
    fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;
}
