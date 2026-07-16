//! Per-axis operations traits — what an adapter implements.
//!
//! Distinct from the engine's workflow capability traits
//! (`project::seam::Source` / `Target`), which state what the *engine*
//! calls. Both pairs mirror the same WIT interfaces and disambiguate by
//! module path.
//!
//! Methods are associated functions: one implementation per component,
//! no instance state, deliberately not object-safe.

use std::future::Future;

use omnia_guest::Model;

use crate::registry::Doc;
use crate::seam::{
    Context, Error, Evidence, Input, Lead, MergePhase, Report, SourceMetadata, TargetMetadata,
    WorkingTree,
};

/// Source adapter contract: `metadata`, prose registry, `survey` / `extract`.
///
/// Generic over [`Model`] so native tests bind scripted doubles and the
/// wasm shim binds `WasiModel`.
pub trait Source {
    /// Axis-local adapter name, e.g. `"captures"`.
    const NAME: &'static str;

    /// Resolve-time metadata.
    fn metadata() -> SourceMetadata;

    /// Embedded prose registry.
    fn docs() -> &'static [Doc];

    /// Survey the bound source into a lead set.
    fn survey<P: Model>(
        model: &P, ctx: &Context<'_>,
    ) -> impl Future<Output = Result<Vec<Lead>, Error>> + Send;

    /// Extract one lead's Evidence.
    fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, lead: &Lead,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;
}

/// Target adapter contract: `metadata`, prose registry, `guidance` /
/// `build` / `merge`.
///
/// Generic over [`Model`] so native tests bind scripted doubles and the
/// wasm shim binds `WasiModel`.
pub trait Target {
    /// Axis-local adapter name, e.g. `"vectis"`.
    const NAME: &'static str;

    /// Resolve-time metadata.
    fn metadata() -> TargetMetadata;

    /// Embedded prose registry.
    fn docs() -> &'static [Doc];

    /// Embedded synthesis-guidance prompt.
    fn guidance() -> &'static str;

    /// Build `slice` against the lent working tree.
    fn build<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, inputs: &[Input], tree: &WorkingTree,
    ) -> impl Future<Output = Result<Report, Error>> + Send;

    /// Run one phased merge gate.
    fn merge<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, phase: MergePhase, tree: &WorkingTree,
    ) -> impl Future<Output = Result<Report, Error>> + Send;
}
