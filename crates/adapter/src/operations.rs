//! Per-axis operations traits — what an adapter implements.
//!
//! Distinct from the engine's `project::seam` capability traits, which
//! mirror the same WIT interfaces. Deliberately not object-safe.

use std::future::Future;

use omnia_guest::Model;

use crate::identity::AdapterIdentity;
use crate::registry::Doc;
use crate::seam::{
    BuildContext, Context, Error, Evidence, Input, Lead, MergePhase, Report, SourceMetadata,
    TargetMetadata, Workspace,
};

/// Source adapter contract: `metadata`, prose registry, `survey` / `extract`.
///
/// Generic over [`Model`] so native tests bind scripted doubles and the
/// wasm shim binds `WasiModel`.
pub trait Source {
    /// Compile-time `(name, version)` identity, e.g.
    /// `AdapterIdentity { name: "captures", version: env!("CARGO_PKG_VERSION") }`.
    const IDENTITY: AdapterIdentity;

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
    /// Compile-time `(name, version)` identity, e.g.
    /// `AdapterIdentity { name: "vectis", version: env!("CARGO_PKG_VERSION") }`.
    const IDENTITY: AdapterIdentity;

    /// Resolve-time metadata.
    fn metadata() -> TargetMetadata;

    /// Embedded prose registry.
    fn docs() -> &'static [Doc];

    /// Synthesis-guidance prompt, read by core synthesis. Deterministic
    /// in every current implementor (they ignore `model`), but the WIT
    /// contract already reserves model-consulting guidance, so the
    /// trait threads the backend now.
    fn guidance<P: Model>(
        model: &P, ctx: &Context<'_>,
    ) -> impl Future<Output = Result<String, Error>> + Send;

    /// Build `slice` inside its prepared private workspace.
    fn build<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, inputs: &[Input], context: &BuildContext,
        workspace: &Workspace,
    ) -> impl Future<Output = Result<Report, Error>> + Send;

    /// Run one phased merge gate over a read-only workspace view.
    fn merge<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, phase: MergePhase, workspace: &Workspace,
    ) -> impl Future<Output = Result<Report, Error>> + Send;
}
