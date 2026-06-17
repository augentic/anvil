//! Provenance projection.
//!
//! The provenance view is **not** a persisted file. It is projected on
//! demand from a slice's single `model.yaml` (which carries provenance
//! inline) by `specify slice provenance`. Because the model and its
//! provenance share one source of truth, the two can never drift — the
//! former `model.yaml`-vs-`provenance.yaml` drift gate retires. The
//! projection round-trips against `schemas/slice/provenance.schema.json`
//! so the audit shape stays stable. See [`DECISIONS.md` §"Single
//! slice-model artifact"][model-artifact] and
//! [`crate::slice::model::SliceModel::to_provenance_index`].
//!
//! [model-artifact]: ../../../../DECISIONS.md#single-slice-model-artifact

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use specify_error::{Error, Result};
use specify_model::spec::provenance::RequirementStatus;

use crate::schema::{PROVENANCE_JSON_SCHEMA, validate_serialisable};

/// In-memory provenance view, projected from `model.yaml`.
///
/// Top-level shape is closed; unknown fields are rejected per the
/// matching schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProvenanceIndex {
    /// Stored schema version. Currently `1`; additive fields land
    /// without a bump.
    pub version: u32,
    /// Slice name. MUST match the directory under `.specify/slices/`.
    pub slice: String,
    /// UTC second-precision timestamp at which the projection was
    /// produced. Resolution is to the second so byte-stable diffs
    /// survive reasonably-fast clocks.
    #[serde(with = "specify_error::serde_rfc3339")]
    pub generated_at: Timestamp,
    /// CLI version that produced the projection (e.g. `specify@2.1.0`).
    pub generator: String,
    /// One entry per `REQ-*` requirement in `model.yaml`; order matches
    /// the model's declaration order.
    pub requirements: Vec<ProvenanceRequirement>,
}

/// One row under [`ProvenanceIndex::requirements`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProvenanceRequirement {
    /// Requirement id matching a `REQ-NNN` heading in `spec.md`.
    pub id: String,
    /// Mirrors the `Status:` line on the matching `spec.md` block.
    pub status: RequirementStatus,
    /// Source keys cited on the matching `spec.md` `Sources:` line.
    /// Empty when `status` is `unknown` and `resolution` is
    /// `unknown-no-evidence`.
    pub sources: Vec<String>,
    /// Every `(source, id)` pair synthesis consulted — *not*
    /// only the winning one. Operators auditing a divergence can see
    /// what was dropped.
    pub contributing_claims: Vec<ContributingClaim>,
    /// How synthesis arrived at the requirement's final value. See
    /// [`ProvenanceResolution`] for the closed variant set and meanings.
    pub resolution: ProvenanceResolution,
    /// Optional trace describing how a non-trivial resolution
    /// selected the winning claim. Present only when `resolution` is
    /// [`ProvenanceResolution::AuthorityResolved`] or
    /// [`ProvenanceResolution::PerSliceOverride`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_trace: Option<ResolutionTrace>,
}

/// One contributing-claim entry under
/// [`ProvenanceRequirement::contributing_claims`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ContributingClaim {
    /// Source key (matches a top-level `plan.yaml.sources.<key>`
    /// binding) the claim came from.
    pub source: String,
    /// Claim id within the source's Evidence file (matches
    /// `claims[].id`).
    pub id: String,
    /// Claim kind copied from the source Evidence claim — closed
    /// enum (mirrored from
    /// `schemas/evidence.schema.json#/$defs/claimKind`).
    pub kind: specify_model::evidence::ClaimKind,
    /// Optional single-line claim payload (statement / criterion /
    /// decision body).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Optional `<path>#L<n>` anchor copied from the source Evidence
    /// claim so the operator can open the original line range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional winner marker. `Some(true)` on the entry synthesis
    /// selected; `Some(false)` on entries dropped by authority
    /// resolution; `None` on `agreed` blocks where there is no
    /// winner / loser distinction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<bool>,
}

/// Closed resolution enum for provenance projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ProvenanceResolution {
    /// One contributing claim only.
    SingleSource,
    /// Multiple contributors, identical value.
    SingleValueAgreement,
    /// Default authority ordering broke the tie.
    AuthorityResolved,
    /// Per-slice `authority-override` map picked the winner.
    PerSliceOverride,
    /// No contributing claims (paired with
    /// [`RequirementStatus::Unknown`]).
    UnknownNoEvidence,
    /// Same-authority disagreement with no override (paired with
    /// [`RequirementStatus::Conflict`]).
    TiedConflict,
}

/// Optional resolution trace under [`ProvenanceRequirement::resolution_trace`].
///
/// `step` is the name of the resolution step that broke the tie
/// (e.g. `per-slice-authority-override`,
/// `default-authority-ordering`). The schema keeps the field
/// free-form until the step taxonomy stabilises in v2; the optional
/// `override` map and `winner` source key narrow the audit trail
/// when present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResolutionTrace {
    /// Name of the resolution step that broke the tie.
    pub step: String,
    /// Optional override map consulted at this step — e.g.
    /// `{ criterion: identity-design-notes }`. Stored as raw JSON to
    /// keep the trace shape open while the taxonomy stabilises.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#override: Option<serde_json::Value>,
    /// Optional source key the step selected as the winner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<String>,
}

impl ProvenanceIndex {
    /// Validate `self` against the embedded `schemas/slice/provenance.schema.json`.
    ///
    /// Returns `Ok(())` on a clean validation; otherwise a payload-free
    /// [`Error::Validation`] keyed on the code `"provenance-schema"`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] when the in-memory projection
    /// fails the schema; falls back to [`Error::Diag`] when the value
    /// is not JSON-serialisable (unreachable in normal operation).
    pub fn validate(&self) -> Result<(), Error> {
        validate_serialisable(
            self,
            PROVENANCE_JSON_SCHEMA,
            "provenance-schema",
            "provenance projection conforms to schemas/slice/provenance.schema.json",
            "provenance-schema-serialise",
            "provenance",
        )
    }
}

#[cfg(test)]
mod tests;
