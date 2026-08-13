//! Per-axis operations traits — what an adapter implements.
//!
//! Distinct from the engine's `project::seam` capability traits, which
//! mirror the same WIT interfaces. Deliberately not object-safe.

use std::future::Future;

use omnia_guest::Model;

use crate::identity::AdapterIdentity;
use crate::registry::Doc;
use crate::seam::{
    BuildContext, Context, Error, Evidence, Input, Lead, MergePhase, PhaseFinding, PhaseReport,
    RepairOrigin, Report, SourceInput, SourceMetadata, TargetMetadata, Workspace,
};

/// Source adapter contract: `metadata`, prose registry, `survey` / `extract`.
///
/// Both judgment operations receive the engine-prepared [`SourceInput`]
/// (the context already lends tree-form inputs and carries
/// `source_key`); adapters never recover a source location themselves.
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

    /// Survey the prepared input into a lead set.
    fn survey<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> impl Future<Output = Result<Vec<Lead>, Error>> + Send;

    /// Extract one lead's Evidence from the prepared input.
    fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput, lead: &Lead,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;
}

/// Target adapter contract: `metadata`, prose registry, `guidance` /
/// `build` / `repair` / `verify` / `review` / `merge`.
///
/// The build-loop operations each perform exactly one pass and return
/// one typed [`PhaseReport`]; operation order, repair routing, and
/// budgets are engine policy (RFC-90 D1) — an implementor must not
/// loop, retry, or select its next operation. Generic over [`Model`]
/// so native tests bind scripted doubles and the wasm shim binds
/// `WasiModel`.
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

    /// Generation only: preparation, writers, capture replay. Must not
    /// verify, repair, or run standards remediation. `build` alone
    /// declares outputs and the UI surface.
    fn build<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, inputs: &[Input], context: &BuildContext,
        workspace: &Workspace,
    ) -> impl Future<Output = Result<PhaseReport, Error>> + Send;

    /// One model-assisted check pass over the lent workspace. Receives
    /// no slice — the same pass runs against whatever candidate the
    /// engine lends. Must return empty outputs, no UI surface, and no
    /// continuation replacement.
    fn verify<P: Model>(
        model: &P, ctx: &Context<'_>, workspace: &Workspace,
    ) -> impl Future<Output = Result<PhaseReport, Error>> + Send;

    /// One findings-directed repair pass. `origin` names the engine
    /// gate that supplied `findings` (the deterministic bounded repair
    /// brief); it never selects the next phase.
    fn repair<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, origin: RepairOrigin, findings: &[PhaseFinding],
        continuation: Option<&[u8]>, workspace: &Workspace,
    ) -> impl Future<Output = Result<PhaseReport, Error>> + Send;

    /// One engineering-standards review pass. Must return empty
    /// outputs and no UI surface.
    fn review<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, continuation: Option<&[u8]>,
        workspace: &Workspace,
    ) -> impl Future<Output = Result<PhaseReport, Error>> + Send;

    /// Run one phased merge gate over a read-only workspace view.
    fn merge<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, phase: MergePhase, workspace: &Workspace,
    ) -> impl Future<Output = Result<Report, Error>> + Send;
}
