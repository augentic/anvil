//! The source-adapter operations trait — what an adapter implements.
//!
//! Distinct from [`crate::Source`], the engine provider's import-side
//! capability over the same WIT interface. Deliberately not object-safe.

use std::future::Future;

use omnia_guest::Model;

use crate::registry::Doc;
use crate::seam::{Context, Error, Evidence, SourceInput, SourceMetadata};

/// Source adapter contract: `metadata`, prose registry, `extract`.
///
/// Generic over [`Model`] so native tests bind scripted doubles and the
/// wasm shim binds `WasiModel`.
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
