//! Inert amendment proposals at `planning/proposals/<digest>.yaml`.
//!
//! A proposal has no authority until `emery plan amend --proposal`
//! compare-and-sets its frontiers and applies it.

mod apply;
mod mutate;
mod overlap;

use std::collections::BTreeMap;
use std::path::Path;

pub use apply::{Applied, apply};
use artifacts::atomic::{bytes_write, serialise_yaml};
use artifacts::leads::Lead;
use diagnostics::digest::sha256_hex;
use error::Error;
pub use mutate::{add, amend as amend_tree, present as has_tree, remove};
pub use overlap::author as author_overlap;
use serde::{Deserialize, Serialize};

use super::decomposition::{Decomposition, FocusParent};
use super::execution::collect_events;
use super::model::{Plan, ProfileRef};
use crate::config::Layout;
use crate::journal::{self, EventKind, claim};
use crate::name::SliceName;
use crate::profile::Assessment;
use crate::snapshot::SnapshotId;
use crate::wave::accepted_cid;

/// Wire version stamped on every proposal document.
pub const VERSION: u32 = 1;

/// One validated but unapplied amendment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Proposal {
    /// Refinement Evidence revealed separately acceptable child
    /// boundaries. Live planning artifacts are unchanged until apply.
    Boundary(Boundary),
    /// Runtime ownership overlap. Names a dependency or fan-in repair.
    Ownership(Ownership),
    /// RFC-96 envelope escalation. DTO only; this crate does not apply it.
    Envelope(Envelope),
    /// Conflicting handoff authority. Stops the affected scope; not an
    /// amendment.
    Revision(Revision),
}

/// Boundary-escalation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Boundary {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Leaf whose Evidence triggered the escalation.
    pub failed_leaf: SliceName,
    /// Closed five-dimension assessment from the refinement judgment.
    pub assessment: Assessment,
    /// Bound target profile the assessment was scored against.
    pub profile: ProfileRef,
    /// Why the Evidence supports a split (or an over-envelope leaf).
    pub rationale: String,
    /// Terminal `(source, lead)` pairs named for focused resurvey.
    pub affected: Vec<FocusParent>,
    /// Candidate catalog after focused child-lead merge. Inert.
    pub candidate_leads: Vec<Lead>,
    /// Candidate nearest-domain decomposition. Inert.
    pub candidate_decomposition: Decomposition,
    /// Compare-and-set frontiers application will check.
    pub expected: Frontiers,
}

/// Ownership-overlap payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Ownership {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Nearest domain that can own the repair.
    pub nearest: String,
    /// Explicit order or fan-in leaf that removes the ambiguity.
    pub repair: Repair,
    /// Compare-and-set frontiers application will check.
    pub expected: Frontiers,
}

/// Envelope-escalation payload (RFC-96). Not applied here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Envelope {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Blocking path or semantic dependency.
    pub blocking: String,
    /// Nearest affected domain.
    pub nearest: String,
    /// Profile digest the obstruction was scored against.
    pub profile: ProfileRef,
    /// Compare-and-set frontiers (recorded, not applied).
    pub expected: Frontiers,
}

/// Inert definition-revision request. Not an amendment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Revision {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Conflicting reviewed-handoff digest.
    pub handoff: SnapshotId,
    /// Why the handoff cannot be amended in this change.
    pub detail: String,
    /// Delivery scope the request stops.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<SliceName>,
}

/// Ownership repair named by an overlap proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Repair {
    /// Add a leaf-to-leaf `depends-on` edge.
    DependsOn {
        /// Predecessor leaf (slice name).
        predecessor: SliceName,
        /// Successor leaf that waits on it.
        successor: SliceName,
    },
    /// Insert a fan-in leaf both overlapping siblings depend on.
    FanIn {
        /// New node id.
        id: String,
        /// Terminal slice mapping.
        slice: SliceName,
        /// Bound target.
        target: String,
        /// Overlapping siblings that depend on the fan-in leaf.
        children: Vec<SliceName>,
    },
}

/// Planning and execution frontiers a proposal expects to still hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Frontiers {
    /// Current `leads-digest`.
    pub leads_digest: SnapshotId,
    /// Current `decomposition-digest`.
    pub decomposition_digest: SnapshotId,
    /// Current `discovery-digest`, when bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery_digest: Option<SnapshotId>,
    /// On-disk `plan.yaml` content digest.
    pub plan_digest: SnapshotId,
    /// Per-target accepted CID (or the bound seed CID when none).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub accepted: BTreeMap<String, SnapshotId>,
    /// Committed leaf → wave digest. Empty before any merge.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub committed: BTreeMap<SliceName, SnapshotId>,
    /// Live claimed leaves. Empty outside an execute epoch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<SliceName>,
}

impl Frontiers {
    /// Snapshot live planning and execution frontiers.
    ///
    /// # Errors
    ///
    /// Missing planning digests, journal I/O, or accepted-CID projection.
    pub fn live(layout: Layout<'_>, plan: &Plan) -> Result<Self, Error> {
        let events = collect_events(layout)?;
        let mut accepted = BTreeMap::new();
        for (id, row) in &plan.targets {
            let cid = accepted_cid(layout, &events, id)?.unwrap_or_else(|| row.cid.clone());
            accepted.insert(id.clone(), cid);
        }
        let mut committed = BTreeMap::new();
        for event in &events {
            if let EventKind::TargetMergeWaveCommitted { digest, members, .. } = &event.kind {
                let wave = SnapshotId::parse(digest)?;
                for member in members {
                    committed.insert(member.clone(), wave.clone());
                }
            }
        }
        let claims: Vec<_> =
            claim::project(&events).iter().map(|(slice, _)| slice.clone()).collect();
        Ok(Self {
            leads_digest: plan.leads_digest.clone().ok_or_else(|| Error::Diag {
                code: "plan-leads-digest-missing",
                detail: "plan.yaml has no leads-digest".into(),
            })?,
            decomposition_digest: plan.decomposition_digest.clone().ok_or_else(|| Error::Diag {
                code: "plan-decomposition-digest-missing",
                detail: "plan.yaml has no decomposition-digest".into(),
            })?,
            discovery_digest: plan.discovery_digest.clone(),
            plan_digest: SnapshotId::parse(&Plan::file_digest(layout)?)?,
            accepted,
            committed,
            claims,
        })
    }

    /// Compare-and-set: every recorded frontier still holds.
    ///
    /// # Errors
    ///
    /// `plan-proposal-stale` naming the first drifted frontier.
    pub fn compare(&self, live: &Self) -> Result<(), Error> {
        stale("leads-digest", &self.leads_digest, &live.leads_digest)?;
        stale("decomposition-digest", &self.decomposition_digest, &live.decomposition_digest)?;
        match (&self.discovery_digest, &live.discovery_digest) {
            (Some(want), Some(got)) => stale("discovery-digest", want, got)?,
            (None, None) => {}
            (want, got) => {
                return Err(stale_err(
                    "discovery-digest",
                    &format!("{want:?}"),
                    &format!("{got:?}"),
                ));
            }
        }
        stale("plan-digest", &self.plan_digest, &live.plan_digest)?;
        if self.accepted != live.accepted {
            return Err(stale_err(
                "accepted",
                &format!("{:?}", self.accepted),
                &format!("{:?}", live.accepted),
            ));
        }
        if self.committed != live.committed {
            return Err(stale_err(
                "committed",
                &format!("{:?}", self.committed),
                &format!("{:?}", live.committed),
            ));
        }
        Ok(())
    }
}

fn stale(name: &str, want: &SnapshotId, got: &SnapshotId) -> Result<(), Error> {
    if want == got { Ok(()) } else { Err(stale_err(name, &want.to_string(), &got.to_string())) }
}

fn stale_err(name: &str, want: &str, got: &str) -> Error {
    Error::validation_failed(
        "plan-proposal-stale",
        "amend --proposal compare-and-sets every expected frontier",
        format!("{name} expected `{want}`, live `{got}`"),
    )
}

impl Proposal {
    /// Canonical YAML of this document.
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn canonical_yaml(&self) -> Result<String, Error> {
        serialise_yaml(self)
    }

    /// Content digest of [`Self::canonical_yaml`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        Ok(SnapshotId::from_digest(&sha256_hex(self.canonical_yaml()?.as_bytes())))
    }

    /// Persist this proposal at `planning/proposals/<digest>.yaml`.
    ///
    /// Idempotent: a retained file with the same digest is left in place.
    ///
    /// # Errors
    ///
    /// Serialization or filesystem failures.
    pub fn save(&self, layout: Layout<'_>) -> Result<SnapshotId, Error> {
        let yaml = self.canonical_yaml()?;
        let digest = SnapshotId::from_digest(&sha256_hex(yaml.as_bytes()));
        let dest = layout.proposal_path(&digest);
        if dest.exists() {
            return Ok(digest);
        }
        bytes_write(&dest, yaml.as_bytes())?;
        Ok(digest)
    }

    /// Load the proposal retained at `digest`.
    ///
    /// # Errors
    ///
    /// `plan-proposal-not-found` when the file is absent; YAML or
    /// digest-mismatch failures.
    pub fn load(layout: Layout<'_>, digest: &SnapshotId) -> Result<Self, Error> {
        let path = layout.proposal_path(digest);
        let text = read_proposal(&path)?;
        let proposal: Self = serde_saphyr::from_str(&text)?;
        let got = proposal.digest()?;
        if got != *digest {
            return Err(Error::validation_failed(
                "plan-proposal-malformed",
                "filename digest matches document content",
                format!("file `{digest}` hashes to `{got}`"),
            ));
        }
        Ok(proposal)
    }

    /// Load every proposal under `planning/proposals/`.
    ///
    /// Missing directory is an empty set.
    ///
    /// # Errors
    ///
    /// Filesystem or parse failures.
    pub fn load_all(layout: Layout<'_>) -> Result<Vec<(SnapshotId, Self)>, Error> {
        let dir = layout.proposals_dir();
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(Error::Filesystem {
                    op: "readdir",
                    path: dir,
                    source,
                });
            }
        };
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| Error::Filesystem {
                op: "readdir",
                path: dir.clone(),
                source,
            })?;
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "yaml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
                op: "read",
                path: path.clone(),
                source,
            })?;
            let proposal: Self = serde_saphyr::from_str(&text)?;
            out.push((proposal.digest()?, proposal));
        }
        out.sort_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
        Ok(out)
    }

    /// The newest unapplied boundary proposal for `slice`, when one exists.
    ///
    /// Applied proposals (a `plan.amend.applied` fact) are skipped so
    /// status does not keep parking a consumed digest.
    ///
    /// # Errors
    ///
    /// Filesystem or parse failures from [`Self::load_all`].
    pub fn boundary_for(
        layout: Layout<'_>, slice: &str,
    ) -> Result<Option<(SnapshotId, Boundary)>, Error> {
        let applied = applied_set(layout)?;
        let mut match_ = None;
        for (digest, proposal) in Self::load_all(layout)? {
            if applied.contains(&digest) {
                continue;
            }
            if let Self::Boundary(boundary) = proposal
                && boundary.failed_leaf.as_str() == slice
            {
                match_ = Some((digest, boundary));
            }
        }
        Ok(match_)
    }

    /// Frontiers this proposal compare-and-sets, when it is an amendment.
    #[must_use]
    pub const fn expected(&self) -> Option<&Frontiers> {
        match self {
            Self::Boundary(body) => Some(&body.expected),
            Self::Ownership(body) => Some(&body.expected),
            Self::Envelope(body) => Some(&body.expected),
            Self::Revision(_) => None,
        }
    }
}

fn read_proposal(path: &Path) -> Result<String, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(Error::validation_failed(
                "plan-proposal-not-found",
                "amend --proposal names a retained proposal digest",
                format!("no proposal at {}", path.display()),
            ))
        }
        Err(source) => Err(Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn applied_set(layout: Layout<'_>) -> Result<std::collections::HashSet<SnapshotId>, Error> {
    let events = collect_events(layout)?;
    Ok(events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::PlanAmendApplied { digest } => Some(digest.clone()),
            _ => None,
        })
        .collect())
}

/// Whether `digest` already has a `plan.amend.applied` fact.
#[must_use]
pub fn is_applied(events: &[journal::Event], digest: &SnapshotId) -> bool {
    events.iter().any(|event| {
        matches!(&event.kind, EventKind::PlanAmendApplied { digest: applied } if applied == digest)
    })
}
