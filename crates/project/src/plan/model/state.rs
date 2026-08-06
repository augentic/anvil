//! Plan state: the `Plan` / `Entry` documents and their closed
//! `Status` state enum.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::reconciliation::{AuthorityOverride, Disagreement, Divergence};
use super::source::{SliceSourceBinding, SourceBinding};
use crate::name::{PlanName, SliceName};

/// Lifecycle state of a single entry in [`Plan::entries`].
///
/// workflow collapses the per-entry state machine to three states:
/// `pending` (default after `plan add` / `plan amend`), `in-progress`
/// (written only by `plan advance`), and `done` (written by `slice
/// merge` — the final per-entry transition). Build failures and merge
/// conflicts leave the active entry `in-progress`; v1 has no per-entry
/// `blocked`, `failed`, or `skipped` state.
///
/// The enum is `Copy + Eq + Hash` so it can appear in `HashSet`s,
/// `match` guards, and hash-keyed lookups without clones. Transition
/// table methods live alongside the internal transition kernel.
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
    /// Not yet started. Written by `plan add` / `plan amend` (forward)
    /// and `plan undo <entry>` (reverse from `InProgress`).
    Pending,
    /// Currently being executed. Written by `plan advance` (forward)
    /// and `plan undo <entry>` (reverse from `Done`).
    InProgress,
    /// Completed successfully. Written by `slice merge` (forward
    /// only — `plan undo` walks back to `InProgress` so the slice can
    /// be re-built and re-merged without inventing a `Reopened`
    /// state).
    Done,
}

/// In-memory model of `plan.yaml` (at the repo root).
///
/// A `Plan` is an ordered, dependency-aware list of [`Entry`]s plus
/// a named map of [`Plan::sources`] (local paths or git URLs) that the
/// entries draw from. There is no plan-level lifecycle state: running
/// `emery plan execute` on an authored plan *is* the approval, and
/// "executing" / "drained" are computed from per-entry [`Status`] at
/// read time.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Plan {
    /// Human-readable plan name, e.g. `platform-v2`.
    pub name: PlanName,
    /// Named source bindings referenced by [`Entry::sources`].
    /// Optional in the YAML; defaults to an empty map.
    ///
    /// Each value is a structured [`SourceBinding`] carrying the
    /// kebab-case source adapter name plus exactly one of `path`
    /// (filesystem path or repo location) or `value` (literal payload
    /// supplied directly to the adapter — used by `intent`).
    #[serde(default)]
    pub sources: BTreeMap<String, SourceBinding>,
    /// Ordered list of plan entries. Order is the intended execution
    /// order; `Plan::next_eligible` applies dependency eligibility.
    #[serde(rename = "slices")]
    pub entries: Vec<Entry>,
}

/// One entry in [`Plan::entries`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Entry {
    /// Stable identifier (kebab-case) unique within the plan.
    pub name: SliceName,
    /// Target registry project. Optional on disk: an omitted value
    /// resolves to the sole project in the topology (a single regular
    /// project synthesised from `project.yaml`), so single-project
    /// plans need not repeat the project name; multi-project workspace
    /// registries require an explicit value.
    ///
    /// The target adapter (`name[@vN]`) is **not** stored on the slice —
    /// it is resolved on demand from this project via the topology
    /// (the committed `.emery/topology.lock` for a workspace, `project.yaml.adapter` for a single
    /// regular project) by the internal target-resolution kernel.
    #[serde(default)]
    pub project: Option<String>,
    /// Current lifecycle state of this entry.
    pub status: Status,
    /// Names of other plan entries that must reach `done` before this
    /// entry is eligible.
    #[serde(default)]
    pub depends_on: Vec<SliceName>,
    /// (source, lead) bindings (workflow §`Slice.sources`).
    /// Each entry pairs a `source` — referencing a top-level
    /// [`Plan::sources`] entry — with the `lead` from
    /// `discovery.md` that contributed to the slice. The bare-string
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
}

impl Plan {
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
}
