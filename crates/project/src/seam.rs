//! Capability traits over the `emery:adapter` guest-to-guest contract.
//!
//! [`Source`] and [`Target`] mirror the WIT interfaces; their DTOs omit
//! caller-owned fields such as the orchestrator-added source attribution.

pub mod wire;

use std::future::Future;

use artifacts::evidence::{AuthorityClass, Claim};
use serde::{Deserialize, Serialize};
pub use wire::BuildReport;

use crate::snapshot::{CodePatch, SnapshotId};

/// Typed seam failure, mirroring the WIT `types.error` variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The request itself is malformed; retrying unchanged is pointless.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// A filesystem operation failed on the adapter side.
    #[error("io: {0}")]
    Io(String),
    /// A judgment call or answer-handling step failed on the adapter side.
    #[error("internal: {0}")]
    Internal(String),
}

/// One lead surfaced by a survey.
///
/// The shape is the discovery lead minus the envelope `source` key,
/// which the orchestrator stamps. Doubles as the item shape of the
/// generated `survey` judgment-answer schema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Lead {
    /// Stable kebab-case lead identifier, unique only within its
    /// source; identity is the `(source, lead)` pair.
    pub lead: String,
    /// Headline used for cross-source reconciliation.
    pub synopsis: String,
    /// Agent-authored per-lead topic slugs (kebab-case). Empty means
    /// unclassified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
}

/// Evidence returned by an extract, without the caller-owned `lead` key.
///
/// Claims are the typed [`Claim`] mirror of the WIT record; open
/// per-kind fields ride its flattened `extras` map. Doubles as the
/// generated `extract` judgment-answer schema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Evidence {
    /// Document-level authority class of this evidence.
    pub authority: AuthorityClass,
    /// The claims extracted from the source, in answer order.
    pub claims: Vec<Claim>,
}

/// One slice-artifact payload, mirroring the WIT `payload` variant.
///
/// `Path` is the artifact's project-relative location ('/'-separated),
/// resolvable in the adapter guest's `"."` preopen and rendered
/// against the lent workspace's artifact root for spawned agents —
/// never host-absolute. `Body` is the inlined artifact text for
/// non-lent deployments (RFC-55). The cases are exclusive: the engine
/// sends `Path` while every deployment lends a workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    /// Project-relative artifact path ('/'-separated).
    Path(String),
    /// Inlined artifact text when the deployment does not lend a workspace.
    Body(String),
}

/// One slice-artifact input to a build.
///
/// Paths cross the seam typed; adapters render them verbatim and never
/// re-derive the engine's slice layout from prose conventions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// The slice's `proposal.md`.
    Proposal(Payload),
    /// The slice's `design.md`.
    Design(Payload),
    /// The slice's `tasks.md`.
    Tasks(Payload),
    /// One behavioural spec (`specs/<domain>/spec.md`).
    Spec(Payload),
    /// Any additional artifact.
    Other(Payload),
}

/// Deterministic per-slice facts the engine forwards to a build,
/// mirroring the WIT `build-context` record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BuildContext {
    /// Kebab-case adapter names of the slice's bound sources, resolved
    /// from the plan entry's `sources[]` through the plan-level
    /// bindings map. Empty when the slice has no resolvable plan entry.
    pub sources: Vec<String>,
}

/// One prepared private workspace, mirroring the WIT `workspace`
/// record (RFC-87).
///
/// Guests share mount preopens, so no directory handle crosses the
/// seam: `root` is a deployment-local path the receiving side resolves
/// against its own preopens (or opens directly off-wasm), and
/// `artifacts` is the agent-visible read-only artifact root for
/// prompts that reference change-tree artifacts from inside a lent
/// workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    /// Opaque identity of the preparation.
    pub id: String,
    /// Deployment-local path of the writable workspace root.
    pub root: String,
    /// Agent-visible read-only artifact root (the project tree).
    pub artifacts: String,
}

/// Which side of the deterministic core merge a `merge` dispatch runs
/// on, mirroring the WIT `merge-phase` enum. The engine's merge stays
/// deterministic; the target's judgment brackets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum MergePhase {
    /// Before the deterministic commit: a blocking finding aborts the
    /// merge with the slice still `built`.
    Preflight,
    /// After the commit and archive: a blocking finding is a terminal
    /// diagnostic, never a rollback.
    Postflight,
}

/// Plan-time discovery and slice-time extraction for a source adapter.
pub trait Source: Send + Sync {
    /// Lightly survey the source into a lead set.
    fn survey(&self, id: String) -> impl Future<Output = Result<Vec<Lead>, Error>> + Send;

    /// Thoroughly extract evidence from the source for one lead.
    fn extract(
        &self, id: String, lead: Lead,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;
}

/// Synthesis guidance, slice builds, and phased merge gates for a
/// target adapter.
pub trait Target: Send + Sync {
    /// Guidance on the expected build artifacts for this target.
    fn guidance(&self, id: String) -> impl Future<Output = Result<String, Error>> + Send;

    /// Build `slice` inside its prepared private workspace.
    fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, context: BuildContext,
        workspace: Workspace,
    ) -> impl Future<Output = Result<BuildReport, Error>> + Send;

    /// Run one target-specific merge gate (`phase`) around the engine's
    /// deterministic core merge. Dispatched twice per slice merge —
    /// preflight before the commit, postflight after it — each over a
    /// read-only view of the built result snapshot.
    fn merge(
        &self, id: String, slice: String, phase: MergePhase, workspace: Workspace,
    ) -> impl Future<Output = Result<BuildReport, Error>> + Send;
}

/// The host-owned private-workspace capability (RFC-87).
///
/// Mirrors the WIT `workspaces` interface: immutable snapshots in a
/// content-addressed store, disposable private workspaces, and code
/// patches derived by comparing trees.
///
/// Implemented by the same providers that carry the other seam
/// capabilities: the native provider calls the
/// [`crate::workspace`] kernel in-process; the engine guest maps the
/// host-implemented WIT imports.
pub trait Workspaces: Send + Sync {
    /// Freeze the product tree (the project root minus VCS and
    /// change-tree state) as an immutable snapshot. Refine records the
    /// result as the slice's target-base pin in `base.yaml`; build
    /// prepares from that recorded pin and must not call this (RFC-86
    /// D25 / D27).
    fn freeze(&self) -> impl Future<Output = Result<SnapshotId, Error>> + Send;

    /// Materialize `base` into a fresh private workspace.
    /// `writable: false` prepares a read-only source view — same
    /// preparation, discarded without capture.
    fn prepare(
        &self, base: SnapshotId, writable: bool,
    ) -> impl Future<Output = Result<Workspace, Error>> + Send;

    /// Capture the workspace's result tree: store and verify every
    /// object, record the result snapshot, and derive the touched
    /// paths against the recorded base.
    fn capture(&self, id: String) -> impl Future<Output = Result<CodePatch, Error>> + Send;

    /// Discard a workspace. Idempotent; captured snapshots survive by
    /// digest.
    fn discard(&self, id: String) -> impl Future<Output = Result<(), Error>> + Send;

    /// Interim code delivery (pre-RFC-89): write `patch`'s touched
    /// paths from its result snapshot onto the product tree, leaving
    /// everything else untouched. Deleted when publication sets own
    /// the final seal.
    fn apply(&self, patch: CodePatch) -> impl Future<Output = Result<(), Error>> + Send;
}

/// The borrowed capability bundle one orchestration run dispatches
/// across: model judgment, source-axis seam, target-axis seam, and
/// adapter resolver.
///
/// The four capabilities stay independent type parameters so tests
/// bind independent mocks per seam; [`Capabilities::provider`] bundles
/// one provider that satisfies all four.
#[derive(Debug)]
pub struct Capabilities<'a, P, S, T, R> {
    /// Judgment-leg model dispatch.
    pub model: &'a P,
    /// Source-axis seam (survey / extract).
    pub sources: &'a S,
    /// Target-axis seam (guidance / build / merge).
    pub targets: &'a T,
    /// Adapter resolver.
    pub resolver: &'a R,
}

impl<'a, Provider> Capabilities<'a, Provider, Provider, Provider, Provider> {
    /// Bundle one provider that carries every capability — the
    /// handler-side constructor over `context.provider`.
    pub const fn provider(provider: &'a Provider) -> Self {
        Self {
            model: provider,
            sources: provider,
            targets: provider,
            resolver: provider,
        }
    }
}

impl<'a, P, S, T, R> Capabilities<'a, P, S, T, R> {
    /// Drop the target seam for phases that never dispatch it (plan
    /// authoring surveys and reconciles but builds nothing).
    #[must_use]
    pub const fn sans_targets(self) -> Capabilities<'a, P, S, (), R> {
        Capabilities {
            model: self.model,
            sources: self.sources,
            targets: &(),
            resolver: self.resolver,
        }
    }
}

// Manual `Copy`/`Clone`: the bundle is four shared borrows, copyable
// regardless of whether the capability types themselves are.
impl<P, S, T, R> Clone for Capabilities<'_, P, S, T, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, S, T, R> Copy for Capabilities<'_, P, S, T, R> {}

/// Map a seam dispatch failure onto the wire contract.
///
/// `operation` is the seam method (`survey`, `extract`, `guidance`,
/// `build`, `merge`); `id` is the routed adapter id (e.g.
/// `source:typescript`).
#[must_use]
pub fn seam_failure(operation: &'static str, id: &str, err: &Error) -> error::Error {
    error::Error::Diag {
        code: "seam-dispatch-failed",
        detail: format!("seam `{operation}` dispatch to `{id}` failed: {err}"),
    }
}

/// The exact routed adapter id for a source dispatch
/// (`source:<name>[@<version>]`) — versioned for a package-resolved
/// identity, unversioned for a cache-backed one.
#[must_use]
pub fn source_id(adapter: &crate::adapter::SourceAdapter) -> String {
    crate::adapter::RoutedId::new(
        crate::adapter::Axis::Source,
        adapter.name.clone(),
        adapter.version.clone(),
    )
    .to_string()
}

/// The exact routed adapter id for a target dispatch
/// (`target:<name>[@<version>]`) — versioned for a package-resolved
/// identity, unversioned for a cache-backed one.
#[must_use]
pub fn target_id(adapter: &crate::adapter::TargetAdapter) -> String {
    crate::adapter::RoutedId::new(
        crate::adapter::Axis::Target,
        adapter.name.clone(),
        adapter.version.clone(),
    )
    .to_string()
}
