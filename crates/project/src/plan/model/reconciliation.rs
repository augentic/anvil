//! Reconciliation and authority concerns: the divergence taxonomy,
//! recorded disagreements, and the per-slice authority override map.

use std::collections::BTreeMap;

use artifacts::evidence::ClaimKind;
use serde::{Deserialize, Serialize};

/// Slice-level reconciliation outcome.
///
/// Closed `none | likely | accepted | rejected` taxonomy on
/// `plan.yaml.slices[].divergence`, written only by `emery plan
/// amend`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Deserialize,
    Serialize,
    strum::Display,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Divergence {
    /// No divergence — the implicit default for slice records (absent
    /// on disk) and the explicit first value of the journal
    /// `plan.amend.divergence` `from` field on the first transition.
    #[serde(rename = "none")]
    None,
    /// Staged by the `/emery:plan` agent after the reconcile write, via
    /// `emery plan amend --divergence likely`, on
    /// materially-disagreeing lead synopses.
    Likely,
    /// Operator-recorded during plan review — divergence acknowledged
    /// and accepted into the plan.
    Accepted,
    /// Operator-recorded during plan review — divergence rejected; the
    /// plan must be re-proposed before execution.
    Rejected,
}

impl Divergence {
    /// Whether a slice carrying this flag must record its disagreeing
    /// values. `Likely` / `Accepted` are live divergences the agent or
    /// operator has affirmed; `None` / `Rejected` carry no obligation.
    #[must_use]
    pub(crate) const fn requires_values(self) -> bool {
        matches!(self, Self::Likely | Self::Accepted)
    }
}

/// One field on which a slice's matched leads materially disagree.
///
/// Recorded by the `/emery:plan` propose agent alongside a `divergence`
/// flag. The CLI never decides materiality — it only checks structural
/// consistency: a flagged slice records at least one disagreement, and
/// each disagreement names at least two distinct source values.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Disagreement {
    /// The aspect the sources disagree on (a free-form label, e.g.
    /// `password-min-length`).
    pub field: String,
    /// The per-source values that disagree on `field`. A genuine
    /// disagreement records at least two distinct source values.
    pub values: Vec<DisagreementValue>,
}

/// One source's value for a [`Disagreement`] field.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DisagreementValue {
    /// Source key contributing this value (a `plan.yaml.sources.<key>`).
    pub source: String,
    /// The value this source surfaced for the disagreeing field.
    pub value: String,
}

/// Per-slice authority override map keyed by claim kind, valued by
/// source key.
///
/// Scoped to one [`super::Entry`]; values MUST be present in the
/// owning slice's [`super::Entry::sources`] list (validation refuses
/// orphans with `slice-authority-override-orphan-source`).
/// `#[serde(transparent)]` over `BTreeMap`: empty map and missing
/// field round-trip identically, leaving the workflow default ordering.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct AuthorityOverride {
    /// Inner map. `BTreeMap` for byte-stable diffs on serialise.
    pub by_kind: BTreeMap<ClaimKind, String>,
}

impl AuthorityOverride {
    pub(super) fn is_empty(&self) -> bool {
        self.by_kind.is_empty()
    }
}
