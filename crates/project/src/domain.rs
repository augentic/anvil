//! Durable domain-convergence rounds at
//! `.emery/change/targets/<target>/domains/<digest>.yaml` — one
//! content-addressed `DomainRound` per round (RFC-96 D8).

use std::collections::BTreeMap;
use std::path::PathBuf;

use diagnostics::digest::sha256_hex;
use error::Error;
use serde::{Deserialize, Serialize};

use crate::config::Layout;
use crate::journal::FactEpochRef;
use crate::snapshot::SnapshotId;

mod closure;

pub use closure::{Closure, protected_closure};

/// Wire version stamped into every domain-round document.
pub const VERSION: u32 = 1;

/// Round kind: frontier composes and verifies a candidate; complete
/// verifies the accepted tree once every child and dependency landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoundKind {
    /// Same-base child patches composed and verified pre-merge.
    Frontier,
    /// Accepted-tree verification after every child completes.
    Complete,
}

/// Closed round verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    /// Domain-level verification passed (or aggregated all-passed).
    Passed,
    /// A blocking finding or failed child verdict.
    Failed,
}

/// One durable domain-convergence round (RFC-96 D8). Unknown fields
/// are rejected; the on-disk name is the content digest of the
/// canonical YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DomainRound {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Decomposition node id this round converged.
    pub domain: String,
    /// Round kind.
    pub kind: RoundKind,
    /// Round verdict.
    pub verdict: Verdict,
    /// Bound targets in canonical order (singular for verified
    /// rounds; plural on a multi-target aggregate).
    pub targets: Vec<String>,
    /// Canonical digest of the `decomposition.yaml` revision.
    pub revision: SnapshotId,
    /// Authorization epoch anchoring this round.
    pub authorization: FactEpochRef,
    /// Per-target base tree identity (wave base for frontier, the
    /// accepted CID for complete).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bases: BTreeMap<String, SnapshotId>,
    /// Child inputs: member build-record digests (frontier) or child
    /// domain-record digests (aggregates).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SnapshotId>,
    /// Patch or committed-wave chain digests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waves: Vec<SnapshotId>,
    /// Per-target result CIDs (the composed candidate for frontier,
    /// the verified accepted tree for complete).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub results: BTreeMap<String, SnapshotId>,
    /// Digest of the protected-input closure ([`Closure::digest`]).
    pub protected_inputs: SnapshotId,
    /// Digest of the domain-level verification report, when
    /// verification ran (absent on aggregates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_report: Option<SnapshotId>,
}

impl DomainRound {
    /// Canonical YAML bytes the content digest covers.
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn canonical_yaml(&self) -> Result<String, Error> {
        artifacts::atomic::serialise_yaml(self)
    }

    /// Content digest of [`Self::canonical_yaml`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        Ok(SnapshotId::from_digest(&sha256_hex(self.canonical_yaml()?.as_bytes())))
    }

    /// Whether this round is the same convergence attempt as `other`:
    /// every identity field matches — verdict, the verification
    /// report, and the (per-run) authorization anchor may differ, so
    /// restart reuses the record across epochs.
    #[must_use]
    pub fn same_key(&self, other: &Self) -> bool {
        self.domain == other.domain
            && self.kind == other.kind
            && self.targets == other.targets
            && self.revision == other.revision
            && self.bases == other.bases
            && self.children == other.children
            && self.waves == other.waves
    }

    /// Persist under every bound target's `domains/` directory
    /// (identical bytes, one content digest). Write-once: an existing
    /// identical file is idempotent.
    ///
    /// # Errors
    ///
    /// Serialization and filesystem failures.
    pub fn write(&self, layout: Layout<'_>) -> Result<SnapshotId, Error> {
        let yaml = self.canonical_yaml()?;
        let digest = SnapshotId::from_digest(&sha256_hex(yaml.as_bytes()));
        for target in &self.targets {
            let path = round_path(layout, target, &digest);
            if path.is_file() {
                continue;
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|source| Error::Filesystem {
                    op: "create-dir",
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            artifacts::atomic::bytes_write(&path, yaml.as_bytes())?;
        }
        Ok(digest)
    }

    /// Load every round persisted for `target`, unordered.
    ///
    /// # Errors
    ///
    /// Read and parse failures; an absent directory is empty.
    pub fn load_all(layout: Layout<'_>, target: &str) -> Result<Vec<Self>, Error> {
        let dir = layout.target_domains_dir(target);
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut rounds = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|source| Error::Filesystem {
            op: "read-dir",
            path: dir.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| Error::Filesystem {
                op: "read-dir",
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
            rounds.push(serde_saphyr::from_str(&text)?);
        }
        Ok(rounds)
    }

    /// The recorded round matching `key` ([`Self::same_key`]), when
    /// one exists.
    ///
    /// # Errors
    ///
    /// Read and parse failures.
    pub fn find(layout: Layout<'_>, target: &str, key: &Self) -> Result<Option<Self>, Error> {
        Ok(Self::load_all(layout, target)?.into_iter().find(|round| round.same_key(key)))
    }
}

/// On-disk path of one round for `target`.
fn round_path(layout: Layout<'_>, target: &str, digest: &SnapshotId) -> PathBuf {
    layout.target_domains_dir(target).join(format!("{}.yaml", digest.digest()))
}

/// Whether the RFC-96 D8 drain gate passes for `target`.
///
/// Passes when a *passed* complete round exists for the root domain
/// at exactly this decomposition revision and accepted CID. Plans
/// without a decomposition (or a target with no accepted CID yet)
/// gate nothing.
///
/// # Errors
///
/// Round read failures.
pub fn complete_passed(
    layout: Layout<'_>, root: &str, revision: &SnapshotId, target: &str,
    accepted: Option<&SnapshotId>,
) -> Result<bool, Error> {
    let Some(accepted) = accepted else {
        return Ok(true);
    };
    let rounds = DomainRound::load_all(layout, target)?;
    Ok(rounds.iter().any(|round| {
        round.kind == RoundKind::Complete
            && round.verdict == Verdict::Passed
            && round.domain == root
            && round.revision == *revision
            && round.results.get(target) == Some(accepted)
    }))
}
