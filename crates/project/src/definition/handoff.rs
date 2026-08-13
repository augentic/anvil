//! Closed RFC-104 D10 handoff DTO and its format-independent digest.

use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use serde::{Deserialize, Serialize};

use crate::snapshot::SnapshotId;

/// Wire version stamped into every handoff.
pub const VERSION: u32 = 1;

/// Reserved evidence-scope source key for inline intent.
pub const INTENT: &str = "intent";

/// Immutable wave projection consumed by detached `plan author`.
///
/// Unknown fields are rejected. The canonical digest covers
/// schema-validated content and is independent of YAML formatting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Handoff {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Definition identity the wave was projected from.
    pub definition: String,
    /// Canonical digest of `scope.yaml`.
    pub scope_digest: SnapshotId,
    /// Canonical digest of `coverage.yaml`.
    pub coverage_digest: SnapshotId,
    /// Canonical digest of `sources.yaml`.
    pub sources_digest: SnapshotId,
    /// Canonical digest of the system model.
    pub system_model_digest: SnapshotId,
    /// Canonical digest of the migration plan.
    pub migration_plan_digest: SnapshotId,
    /// Selected wave projection.
    pub wave: Wave,
}

/// Wave block inside a [`Handoff`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Wave {
    /// Wave id selected for delivery.
    pub id: String,
    /// Canonical digest of the migration-plan wave record.
    pub digest: SnapshotId,
    /// Bounded outcome the wave delivers.
    pub outcome: String,
    /// Architecture state before and after the wave.
    pub architecture: Architecture,
    /// Reviewed logical targets with origin locators and adapter pins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<Target>,
    /// Source values, adapters, leads, and Evidence that may inform delivery.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_scopes: Vec<Scope>,
    /// Reviewed source-to-target assignments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delivery_mappings: Vec<Mapping>,
    /// Elements that may experience an observable consequence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_elements: Vec<String>,
    /// Elements in the delivery ownership envelope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touched_elements: Vec<String>,
    /// Read-only architectural context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_elements: Vec<String>,
    /// Predecessor waves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Ref>,
    /// External preconditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<Ref>,
    /// Modernization dispositions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dispositions: Vec<Ref>,
    /// State-movement records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_movements: Vec<Ref>,
    /// Coexistence records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coexistence: Vec<Ref>,
    /// Cutover records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cutover: Vec<Ref>,
    /// Rollback records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rollback: Vec<Ref>,
    /// Operational-readiness records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operational_readiness: Vec<Ref>,
    /// Acceptance records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance: Vec<Ref>,
    /// Verification records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verification: Vec<Ref>,
    /// Conservation records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conservation: Vec<Ref>,
    /// Material unknowns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<Ref>,
    /// Commercial assumptions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<Ref>,
    /// Authority decisions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<Ref>,
}

/// Architecture ids before and after the wave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Architecture {
    /// Architecture revision the wave starts from.
    pub before: Ref,
    /// Architecture revision the wave produces.
    pub after: Ref,
}

/// One reviewed delivery target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Target {
    /// Logical target id.
    pub id: String,
    /// Mutable origin locator; RFC-88 resolves it to an exact revision and CID.
    pub locator: String,
    /// Exact target-adapter package pin.
    pub adapter: String,
}

/// One evidence scope that may inform delivery.
///
/// Closes `value` xor `source-cid`: `intent` carries inline `value`;
/// every other source carries `source-cid`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Scope {
    /// Source key (`intent` is reserved).
    pub source: String,
    /// Tree identity of a location-backed source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_cid: Option<SnapshotId>,
    /// Inline value; required for [`INTENT`], forbidden otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Exact source-adapter package pin.
    pub adapter: String,
    /// Lead id within `source`.
    pub lead: String,
    /// Digest of the extracted Evidence document.
    pub evidence_digest: SnapshotId,
}

/// Reviewed source-to-target assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Mapping {
    /// Source key.
    pub source: String,
    /// Lead id within `source`.
    pub lead: String,
    /// Target id in [`Wave::targets`].
    pub target: String,
}

/// `{ id, digest }` reference to one canonical record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Ref {
    /// Stable record id.
    pub id: String,
    /// Canonical digest of that record.
    pub digest: SnapshotId,
}

impl Handoff {
    /// Parse YAML, reject unknown fields, and enforce closed invariants.
    ///
    /// # Errors
    ///
    /// `definition-handoff-malformed` on YAML/unknown-field failures;
    /// `definition-handoff-version`, `definition-scope-xor`, and
    /// `definition-intent-form` on closed-shape violations.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let handoff: Self = serde_saphyr::from_str(text).map_err(|err| Error::Diag {
            code: "definition-handoff-malformed",
            detail: err.to_string(),
        })?;
        handoff.validate()?;
        Ok(handoff)
    }

    /// Load and validate a handoff file.
    ///
    /// # Errors
    ///
    /// Filesystem failures; the same closed-shape errors as [`Self::parse`].
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Canonical YAML bytes (trailing newline, stable field order).
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn canonical_yaml(&self) -> Result<String, Error> {
        artifacts::atomic::serialise_yaml(self)
    }

    /// Content digest of [`Self::canonical_yaml`] as a [`SnapshotId`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        Ok(SnapshotId::from_digest(&sha256_hex(self.canonical_yaml()?.as_bytes())))
    }

    /// Enforce version, non-empty identities, and evidence-scope xor.
    ///
    /// # Errors
    ///
    /// Typed `definition-*` diagnostics listed on [`Self::parse`].
    pub fn validate(&self) -> Result<(), Error> {
        if self.version != VERSION {
            return Err(Error::Diag {
                code: "definition-handoff-version",
                detail: format!("handoff version `{}` is not `{VERSION}`", self.version),
            });
        }
        if self.definition.is_empty() {
            return Err(Error::Diag {
                code: "definition-handoff-malformed",
                detail: "handoff `definition` must be non-empty".into(),
            });
        }
        if self.wave.id.is_empty() {
            return Err(Error::Diag {
                code: "definition-handoff-malformed",
                detail: "handoff `wave.id` must be non-empty".into(),
            });
        }
        for scope in &self.wave.evidence_scopes {
            check_scope(scope)?;
        }
        Ok(())
    }
}

fn check_scope(scope: &Scope) -> Result<(), Error> {
    let has_value = scope.value.as_ref().is_some_and(|value| !value.is_empty());
    let has_cid = scope.source_cid.is_some();
    match (has_value, has_cid) {
        (true, false) | (false, true) => {}
        (false, false) => {
            return Err(Error::Diag {
                code: "definition-scope-xor",
                detail: format!(
                    "evidence scope `{}` must carry `value` or `source-cid`",
                    scope.source
                ),
            });
        }
        (true, true) => {
            return Err(Error::Diag {
                code: "definition-scope-xor",
                detail: format!(
                    "evidence scope `{}` must not carry both `value` and `source-cid`",
                    scope.source
                ),
            });
        }
    }
    let is_intent = scope.source == INTENT;
    if is_intent && has_cid {
        return Err(Error::Diag {
            code: "definition-intent-form",
            detail: "evidence scope `intent` carries `value` and no `source-cid`".into(),
        });
    }
    if !is_intent && has_value {
        return Err(Error::Diag {
            code: "definition-intent-form",
            detail: format!(
                "evidence scope `{}` is location-backed and carries `source-cid`, not `value`",
                scope.source
            ),
        });
    }
    Ok(())
}
