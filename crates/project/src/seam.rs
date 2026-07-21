//! Capability traits over the `specify:adapter` guest-to-guest contract.
//!
//! [`Source`] and [`Target`] mirror the WIT `source` / `target`
//! interfaces. Their DTOs omit caller-owned fields, such as the source
//! attribution added by the orchestrator. Keeping `wit-bindgen` providers
//! outside this crate leaves engine code wasm-free.
//!
//! [`Capabilities`] is the borrowed capability bundle the guest
//! orchestrations in the slice and change crates dispatch across.

pub mod wire;

use std::future::Future;

use artifacts::evidence::{AuthorityClass, Claim};
use serde::{Deserialize, Serialize};
pub use wire::BuildReport;

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

/// One slice-artifact input to a build.
///
/// Bodies cross the seam directly; adapters read other context through
/// their shared-mount preopen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// The slice's `proposal.md` body.
    Proposal(String),
    /// The slice's `design.md` body.
    Design(String),
    /// The slice's `tasks.md` body.
    Tasks(String),
    /// One behavioural spec body (`specs/<domain>/spec.md`).
    Spec(String),
    /// Any additional artifact body.
    Other(String),
}

/// Names the tree a build operates on.
///
/// Guests share mount preopens, so no directory handle crosses the seam.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingTree {
    /// The snapshot the operation applies against.
    pub base: String,
    /// Optional path beneath the shared mount root.
    pub subpath: Option<String>,
}

impl WorkingTree {
    /// The live shared mount every build applies against.
    #[must_use]
    pub fn live() -> Self {
        Self {
            base: "live".to_string(),
            subpath: None,
        }
    }
}

/// Which side of the deterministic core merge a `merge` dispatch runs
/// on, mirroring the WIT `merge-phase` enum. The engine's merge stays
/// deterministic; the target's judgment brackets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergePhase {
    /// Before the deterministic commit: a blocking finding aborts the
    /// merge with the slice still `built`.
    Preflight,
    /// After the commit and archive: a blocking finding is a terminal
    /// diagnostic, never a rollback.
    Postflight,
}

impl std::fmt::Display for MergePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Preflight => "preflight",
            Self::Postflight => "postflight",
        })
    }
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

    /// Build `slice` against the shared project mount.
    fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, tree: WorkingTree,
    ) -> impl Future<Output = Result<BuildReport, Error>> + Send;

    /// Run one target-specific merge gate (`phase`) around the engine's
    /// deterministic core merge. Dispatched twice per slice merge —
    /// preflight before the commit, postflight after it.
    fn merge(
        &self, id: String, slice: String, phase: MergePhase, tree: WorkingTree,
    ) -> impl Future<Output = Result<BuildReport, Error>> + Send;
}

/// The borrowed capability bundle one orchestration run dispatches
/// across: model judgment, source-axis seam, target-axis seam, and
/// adapter resolver.
///
/// The four capabilities stay independent type parameters so tests
/// bind independent mocks per seam; the shipped provider satisfies
/// all four at once, so handlers bundle it with
/// [`Capabilities::provider`]. Phases that use a subset simply leave
/// the unused parameter unbounded (plan authoring never dispatches
/// the target seam).
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

/// The plan-bound adapter id routing a source dispatch
/// (`source:<adapter>`).
#[must_use]
pub fn source_id(adapter: &str) -> String {
    format!("{}:{adapter}", crate::adapter::Axis::Source.prefix())
}

/// The plan-bound adapter id routing a target dispatch
/// (`target:<name>`).
#[must_use]
pub fn target_id(name: &str) -> String {
    format!("{}:{name}", crate::adapter::Axis::Target.prefix())
}
