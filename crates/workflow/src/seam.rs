//! The adapter seam: capability traits over the `augentic:specify`
//! guest-to-guest contract (RFC-61 Step 4, Milestone C).
//!
//! [`SourceSeam`] and [`TargetSeam`] mirror the WIT `source` / `target`
//! interfaces the workflow guest imports; the DTOs below mirror the WIT
//! records minus what the caller already knows (a survey's leads carry
//! no `source` — the orchestrator attributes them; a build's report
//! reuses the canonical [`BuildReport`] wire shape, which the Milestone
//! D shim widens the compact WIT `report` into). Orchestrators in
//! [`crate::orchestrate`] take `&impl SourceSeam` / `&impl TargetSeam`
//! bounds, so the crate stays wasm-free: the `wit-bindgen`-backed
//! providers arrive with the guest shim (Milestone D), and native tests
//! bind the scripted [`MockSourceSeam`] / [`MockTargetSeam`].
//!
//! The trait shape follows `specify-guest-model`'s `Model`: bare
//! `impl Future + Send` method signatures and a typed [`Error`]
//! mirroring the WIT `types.error` variant. Unlike `Model` there is no
//! `wasm32` default body — the WIT binding is the Milestone D shim's
//! concern — so a single declaration serves every target.

#[cfg(not(target_arch = "wasm32"))]
mod mock;

use std::future::Future;

use serde_json::Value as JsonValue;
use specify_model::evidence::AuthorityClass;

#[cfg(not(target_arch = "wasm32"))]
pub use self::mock::{MockSourceSeam, MockTargetSeam, SourceCall, TargetCall};
use crate::slice::BuildReport;

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

/// One lead surfaced by a survey — mirrors the WIT `source.lead` record.
///
/// The shape is `schemas/discovery/lead.schema.json` minus the envelope
/// `source` key, which the orchestrator stamps (the surveying source
/// owns attribution).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lead {
    /// Stable kebab-case lead identifier, unique only within its
    /// source; identity is the `(source, lead)` pair.
    pub lead: String,
    /// A reconciliation-grade per-source headline of the lead.
    pub synopsis: String,
    /// Agent-authored per-lead topic slugs (kebab-case). Empty means
    /// unclassified.
    pub topics: Vec<String>,
}

/// The evidence returned by an extract — mirrors the WIT
/// `source.evidence` record: `schemas/evidence.schema.json` minus the
/// envelope `lead` key, which the extract call names.
///
/// Claims ride as raw JSON values rather than a closed struct: the
/// evidence schema leaves per-kind body fields open
/// (`additionalProperties: true`), so a typed mirror would silently
/// drop fields synthesis reads verbatim (e.g. `example`'s
/// `replay-digest`). The orchestrator composes the full document and
/// schema-gates it before anything becomes visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// Document-level authority class of this evidence.
    pub authority: AuthorityClass,
    /// The claims extracted from the source, in answer order.
    pub claims: Vec<JsonValue>,
}

/// One slice-artifact input to a build — mirrors the WIT `target.input` variant.
///
/// Each carries the artifact body, not a path: no descriptor crosses
/// the seam, and the adapter guest reads further context through its
/// own shared-mount preopen.
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

/// Names the tree a build operates on — mirrors the WIT `working-tree` record.
///
/// Every guest shares the same mount preopens, so no directory handle
/// crosses the seam: `base` names the snapshot the operation applies
/// against and `subpath` optionally scopes it beneath the shared mount
/// root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingTree {
    /// The snapshot the operation applies against.
    pub base: String,
    /// Optional path beneath the shared mount root.
    pub subpath: Option<String>,
}

/// The source axis of the seam: plan-time lead discovery and slice-time
/// Evidence extraction, routed to the exporting adapter guest by the
/// plan-bound `id` (e.g. `source:typescript`).
pub trait SourceSeam: Send + Sync {
    /// Lightly survey the source into a lead set.
    fn survey(&self, id: String) -> impl Future<Output = Result<Vec<Lead>, Error>> + Send;

    /// Thoroughly extract evidence from the source for one lead.
    fn extract(
        &self, id: String, lead: Lead,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;
}

/// The target axis of the seam: synthesis guidance and the slice build,
/// routed to the exporting adapter guest by the plan-bound `id` (e.g.
/// `target:omnia`).
///
/// Deliberately no `merge` method: the WIT contract carries one, but
/// native never dispatches a target merge brief today, and the guest
/// merge orchestrator stays deterministic-only to match (RFC-61 Step 4
/// decision D2).
pub trait TargetSeam: Send + Sync {
    /// Guidance on the expected build artifacts for this target, read
    /// by synthesis in place of the manifest `shape` brief.
    fn guidance(&self, id: String) -> impl Future<Output = Result<String, Error>> + Send;

    /// Build `slice` against the shared project mount. The report is
    /// the canonical [`BuildReport`] wire shape (envelope keys
    /// included); the Milestone D shim widens the compact WIT `report`
    /// into it, so the orchestrator's finalize tail runs the same
    /// schema gate and enforcement the native verb does.
    fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, tree: WorkingTree,
    ) -> impl Future<Output = Result<BuildReport, Error>> + Send;
}
