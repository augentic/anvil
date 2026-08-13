//! Inert amendment proposals at `planning/proposals/<digest>.yaml`.
//!
//! Step 16 lands the boundary shape and persist path; application
//! (`plan amend --proposal`) is a later step.

use std::collections::BTreeMap;

use artifacts::atomic::{bytes_write, serialise_yaml};
use artifacts::leads::Lead;
use diagnostics::digest::sha256_hex;
use error::Error;
use serde::{Deserialize, Serialize};

use super::decomposition::{Decomposition, FocusParent};
use super::model::ProfileRef;
use crate::config::Layout;
use crate::name::SliceName;
use crate::profile::Assessment;
use crate::snapshot::SnapshotId;

/// Wire version stamped on every proposal document.
pub const VERSION: u32 = 1;

/// One validated but unapplied amendment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Proposal {
    /// Refinement Evidence revealed separately acceptable child
    /// boundaries. Live `leads.md` / `decomposition.yaml` / `plan.yaml`
    /// are unchanged until application.
    Boundary(Boundary),
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
    /// # Errors
    ///
    /// Filesystem or parse failures from [`Self::load_all`].
    pub fn boundary_for(
        layout: Layout<'_>, slice: &str,
    ) -> Result<Option<(SnapshotId, Boundary)>, Error> {
        let mut match_ = None;
        for (digest, proposal) in Self::load_all(layout)? {
            if let Self::Boundary(boundary) = proposal
                && boundary.failed_leaf.as_str() == slice
            {
                match_ = Some((digest, boundary));
            }
        }
        Ok(match_)
    }
}
