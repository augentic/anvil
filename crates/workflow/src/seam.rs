//! Capability traits over the `specify:adapter` guest-to-guest contract.
//!
//! [`SourceSeam`] and [`TargetSeam`] mirror the WIT `source` / `target`
//! interfaces. Their DTOs omit caller-owned fields, such as the source
//! attribution added by the orchestrator. Keeping `wit-bindgen` providers
//! outside this crate leaves workflow code wasm-free.

use std::future::Future;

use artifacts::evidence::AuthorityClass;
use serde_json::Value as JsonValue;

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

/// One lead surfaced by a survey.
///
/// The shape is `schemas/discovery/lead.schema.json` minus the envelope
/// `source` key, which the orchestrator stamps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lead {
    /// Stable kebab-case lead identifier, unique only within its
    /// source; identity is the `(source, lead)` pair.
    pub lead: String,
    /// Headline used for cross-source reconciliation.
    pub synopsis: String,
    /// Agent-authored per-lead topic slugs (kebab-case). Empty means
    /// unclassified.
    pub topics: Vec<String>,
}

/// Evidence returned by an extract, without the caller-owned `lead` key.
///
/// Claims remain raw JSON because per-kind fields are open; a closed
/// mirror could drop data that synthesis reads verbatim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// Document-level authority class of this evidence.
    pub authority: AuthorityClass,
    /// The claims extracted from the source, in answer order.
    pub claims: Vec<JsonValue>,
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
    pub(crate) fn live() -> Self {
        Self {
            base: "live".to_string(),
            subpath: None,
        }
    }
}

/// Plan-time discovery and slice-time extraction for a source adapter.
pub trait SourceSeam: Send + Sync {
    /// Lightly survey the source into a lead set.
    fn survey(&self, id: String) -> impl Future<Output = Result<Vec<Lead>, Error>> + Send;

    /// Thoroughly extract evidence from the source for one lead.
    fn extract(
        &self, id: String, lead: Lead,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;
}

/// Synthesis guidance and slice builds for a target adapter.
///
/// Merge is omitted because the merge orchestrator is deterministic and
/// does not dispatch to a target.
pub trait TargetSeam: Send + Sync {
    /// Guidance on the expected build artifacts for this target.
    fn guidance(&self, id: String) -> impl Future<Output = Result<String, Error>> + Send;

    /// Build `slice` against the shared project mount.
    fn build(
        &self, id: String, slice: String, inputs: Vec<Input>, tree: WorkingTree,
    ) -> impl Future<Output = Result<BuildReport, Error>> + Send;
}
