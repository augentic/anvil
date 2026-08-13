//! Plan state: the `Plan` / `Entry` documents and the projected
//! [`Status`] ladder labels (RFC-86 D2 / D11).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::binding::{DefinitionIdentity, TargetBinding};
use super::reconciliation::{AuthorityOverride, Disagreement, Divergence};
use super::source::{SliceSourceBinding, SourceBinding};
use crate::name::{PlanName, SliceName};
use crate::snapshot::SnapshotId;

/// Projected per-entry ladder label (RFC-86 D2).
///
/// Not stored on `plan.yaml`. `plan status`, advance eligibility, and
/// undo walk these labels from the fact union: `pending` (default),
/// `in-progress` (advance / live claim), `done` (archive /
/// postflight-failed). The enum stays `Copy + Eq + Hash` for
/// hash-keyed ladder maps and `match` guards.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Status {
    /// Not yet claimed or advanced.
    Pending,
    /// Claimed / advanced; phase work may be in flight.
    InProgress,
    /// Merged (archive or postflight-failed fact).
    Done,
}

/// In-memory model of `plan.yaml` (under the change home).
///
/// A `Plan` is an ordered, dependency-aware list of [`Entry`]s plus
/// the bound delivery topology (`targets` / `sources`) copied from
/// `discovery.yaml`. There is no plan-level lifecycle state and no
/// per-entry stored status — progress projects from artifacts and
/// facts (RFC-86 D2).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Plan {
    /// Human-readable plan name, e.g. `platform-v2`.
    pub name: PlanName,
    /// Canonical digest of `discovery.yaml`. Absent until wave binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery_digest: Option<SnapshotId>,
    /// Canonical digest of `leads.md`. Set when the catalog is imported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leads_digest: Option<SnapshotId>,
    /// Canonical digest of `decomposition.yaml`. Filled by later phases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decomposition_digest: Option<SnapshotId>,
    /// Reviewed-handoff identity. Absent until wave binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition: Option<DefinitionIdentity>,
    /// Bound delivery targets keyed by the persisted target id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub targets: BTreeMap<String, TargetBinding>,
    /// Named source bindings referenced by [`Entry::sources`].
    #[serde(default)]
    pub sources: BTreeMap<String, SourceBinding>,
    /// Ordered list of plan entries. Order is the intended execution
    /// order; eligibility applies dependency + projected ladders.
    #[serde(rename = "slices")]
    pub entries: Vec<Entry>,
}

/// One entry in [`Plan::entries`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Entry {
    /// Stable identifier (kebab-case) unique within the plan.
    pub name: SliceName,
    /// Required key in [`Plan::targets`]. Never omitted.
    pub target: String,
    /// Names of other plan entries that must reach projected `done`
    /// before this entry is eligible.
    #[serde(default)]
    pub depends_on: Vec<SliceName>,
    /// (source, lead) bindings (workflow §`Slice.sources`).
    /// Each entry pairs a `source` — referencing a top-level
    /// [`Plan::sources`] entry — with the `lead` from
    /// the catalog that contributed to the slice. The bare-string
    /// shorthand `<key>` is accepted on the wire as sugar for
    /// `{ source: <key>, lead: <slice.name> }`; in memory we
    /// preserve the on-disk form via [`SliceSourceBinding`].
    #[serde(default)]
    pub sources: Vec<SliceSourceBinding>,
    /// Baseline paths relevant to this change, relative to `.emery/`.
    /// Briefs use these as a focus hint when scanning baseline directories.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,
    /// Free-form human-readable description.
    #[serde(default)]
    pub description: Option<String>,
    /// workflow §Plan-time reconciliation — closed enum capturing slice-level
    /// reconciliation outcome. Absent on disk (the default) is semantic `none`.
    /// `Likely` is set by `/emery:plan`'s `propose` sub-step on
    /// materially-disagreeing lead synopses; `Accepted` /
    /// `Rejected` are written by the operator during plan review via
    /// `emery plan amend --divergence`. Advisory metadata in v1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divergence: Option<Divergence>,
    /// workflow §Plan-time reconciliation — the per-field disagreeing
    /// values backing a `divergence` flag. The `/emery:plan` propose agent
    /// records them when it flags `divergence: likely`; the CLI never
    /// decides materiality, only that a flagged slice records them and a
    /// recorded set carries a flag (`slice-divergence-unrecorded` /
    /// `slice-divergence-orphan-values`). Empty (the default) stays off
    /// disk.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disagreements: Vec<Disagreement>,
    /// per-slice authority override — optional per-slice authority override map keyed
    /// by claim kind, valued by source key. Keys are the closed
    /// [`artifacts::evidence::ClaimKind`] enum; values MUST be source
    /// keys present in this slice's own [`Entry::sources`] list —
    /// orphan keys are rejected by `emery slice validate` with
    /// `slice-authority-override-orphan-source`. Empty map and
    /// missing field are equivalent.
    #[serde(default, skip_serializing_if = "AuthorityOverride::is_empty")]
    pub authority_override: AuthorityOverride,
    /// Authorise a whole-document (`screens:`) slice composition to
    /// overwrite a non-empty baseline when this entry's merge phase
    /// commits. Reserved for intentional full-baseline rewrites (e.g. a
    /// dedicated refactoring slice); routine per-screen edits flow
    /// through `delta:` and never need it. Set via `emery plan amend
    /// <entry> --allow-composition-replace`; read by the execute
    /// loop's merge step. `false` (the default) stays off disk.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_composition_replace: bool,
}

impl Plan {
    /// Empty named plan. Binding and later phases fill topology.
    #[must_use]
    pub fn named(name: impl Into<PlanName>) -> Self {
        Self {
            name: name.into(),
            discovery_digest: None,
            leads_digest: None,
            decomposition_digest: None,
            definition: None,
            targets: BTreeMap::new(),
            sources: BTreeMap::new(),
            entries: Vec::new(),
        }
    }

    /// The shared `plan-entry-not-found` failure for `name`: the
    /// detail lists the plan's entry names so a typo'd entry reads as
    /// a typo, not a missing plan.
    #[must_use]
    pub fn entry_not_found(&self, name: &str) -> error::Error {
        let available: Vec<&str> = self.entries.iter().map(|e| e.name.as_str()).collect();
        let inventory = if available.is_empty() {
            "the plan has no entries".to_string()
        } else {
            format!("available: {}", available.join(", "))
        };
        error::Error::Diag {
            code: "plan-entry-not-found",
            detail: format!("no plan entry named `{name}` ({inventory})"),
        }
    }

    /// The shared `plan-source-unknown` failure for `source`: the
    /// detail lists the plan's bound source keys and reminds the
    /// operator that `verb` resolves keys, not adapter names.
    #[must_use]
    pub fn source_not_found(&self, verb: &str, source: &str) -> error::Error {
        let keys: Vec<&str> = self.sources.keys().map(String::as_str).collect();
        let inventory = if keys.is_empty() {
            "the plan binds no sources".to_string()
        } else {
            format!("bound keys: {}", keys.join(", "))
        };
        error::Error::Diag {
            code: "plan-source-unknown",
            detail: format!(
                "no source `{source}` in plan.yaml.sources ({inventory}); `{verb}` resolves its \
                 argument against the plan's source keys, not the adapter name"
            ),
        }
    }

    /// Look up a required [`Plan::targets`] row.
    ///
    /// # Errors
    ///
    /// `plan-target-unknown` when `key` is absent.
    pub fn target(&self, key: &str) -> Result<&TargetBinding, error::Error> {
        self.targets.get(key).ok_or_else(|| {
            let keys: Vec<&str> = self.targets.keys().map(String::as_str).collect();
            let inventory = if keys.is_empty() {
                "the plan binds no targets".to_string()
            } else {
                format!("bound keys: {}", keys.join(", "))
            };
            error::Error::Diag {
                code: "plan-target-unknown",
                detail: format!("no target `{key}` in plan.yaml.targets ({inventory})"),
            }
        })
    }
}

impl Entry {
    /// A named entry bound to `target`, with empty sources and deps.
    #[must_use]
    pub fn named(name: impl Into<SliceName>, target: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            target: target.into(),
            depends_on: Vec::new(),
            sources: Vec::new(),
            context: Vec::new(),
            description: None,
            divergence: None,
            disagreements: Vec::new(),
            authority_override: AuthorityOverride::default(),
            allow_composition_replace: false,
        }
    }
}
