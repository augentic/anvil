//! The refinement manifest (`refinement.yaml`, RFC-91 D4): the record
//! of one refinement's exact inputs and output bundle, plus the
//! freshness projection; assembly stays in the `slice` crate.

pub mod freshness;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use diagnostics::digest::sha256_hex;
use error::Error;
pub use freshness::{
    Freshness, Live, MISSING_CODE, STALE_CODE, findings, freshness, freshness_with,
    latest_archive, predecessor_digest,
};
use serde::{Deserialize, Serialize};

use crate::snapshot::SnapshotId;

/// Manifest wire version.
pub const VERSION: u32 = 1;

/// On-disk `.emery/slices/<slice>/refinement.yaml`.
///
/// The canonical bytes are the YAML serialization written by
/// [`Manifest::write`]; [`file_digest`] over the untouched on-disk
/// file is the refinement digest downstream identities bind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Manifest {
    /// Wire version ([`VERSION`]).
    pub version: u32,
    /// Owning slice name.
    pub slice: String,
    /// Exact input identities the refinement consumed.
    pub inputs: Inputs,
    /// Every slice artifact the assembled target build request may
    /// consume, with path, kind, and content digest.
    pub bundle: Vec<BundleEntry>,
}

/// The `inputs:` block — one digest per closed refinement input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Inputs {
    /// Slice-local canonical planning projections
    /// ([`crate::plan::Projections`]).
    pub planning: Planning,
    /// RFC-88 profile identity — the canonical empty digest
    /// ([`empty_digest`]) until profiles land.
    pub profile: SnapshotId,
    /// RFC-97 advisory-observation identity — the canonical empty
    /// digest ([`empty_digest`]) until observations land.
    pub observations: SnapshotId,
    /// Digest of the target guidance text consumed by synthesis.
    /// Recorded identity supplied by the refine orchestration; never
    /// recomputed by freshness (guidance is not a file on disk).
    pub target_guidance: SnapshotId,
    /// Tree digest of the immutable `.emery/specs/` baseline read by
    /// synthesis ([`crate::plan::dir_cid`]).
    pub baseline_specs: SnapshotId,
    /// Per-source tree digests copied from the closed plan source set
    /// for every binding on this slice.
    pub sources: BTreeMap<String, SnapshotId>,
    /// Ordered predecessor refinement identities (RFC-91 D3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,
}

/// The three planning digests (`inputs.planning`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Planning {
    /// The leaf's complete plan entry projection.
    pub entry: SnapshotId,
    /// The retained contributing lead closure projection.
    pub leads: SnapshotId,
    /// The decomposition-scope projection.
    pub decomposition: SnapshotId,
}

/// One predecessor's refinement identity under `inputs.dependencies`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Dependency {
    /// Predecessor slice name.
    pub slice: String,
    /// Predecessor refinement digest (its manifest's content digest).
    pub refinement: SnapshotId,
}

/// One covered artifact under `bundle:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BundleEntry {
    /// Slice-relative artifact path.
    pub path: String,
    /// Closed artifact kind.
    pub kind: Kind,
    /// Content digest of the file's raw bytes.
    pub digest: SnapshotId,
}

/// Closed bundle-artifact kind, mirroring the canonical build-input
/// declaration set assembled into the target build request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// `proposal.md`.
    Proposal,
    /// `design.md`.
    Design,
    /// `tasks.md`.
    Tasks,
    /// A per-domain `specs/<domain>/spec.md`.
    Spec,
    /// An adapter-declared additional input.
    Additional,
}

impl Manifest {
    /// Path of the refinement manifest under `slice_dir`.
    #[must_use]
    pub fn path(slice_dir: &Path) -> PathBuf {
        slice_dir.join(crate::slice::REFINEMENT_FILE)
    }

    /// Atomically write this manifest to [`Self::path`].
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization and filesystem failures.
    pub fn write(&self, slice_dir: &Path) -> Result<(), Error> {
        artifacts::atomic::yaml_write(&Self::path(slice_dir), self)
    }

    /// Load a previously written manifest.
    ///
    /// # Errors
    ///
    /// Filesystem / YAML parse failures.
    pub fn load(slice_dir: &Path) -> Result<Self, Error> {
        let path = Self::path(slice_dir);
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        Ok(serde_saphyr::from_str(&text)?)
    }
}

/// Refinement digest of the on-disk manifest at `slice_dir`, hashed
/// over the raw file bytes so a hand edit changes the identity.
/// `None` when no manifest exists.
///
/// # Errors
///
/// Filesystem read failures other than absence.
pub fn file_digest(slice_dir: &Path) -> Result<Option<SnapshotId>, Error> {
    let path = Manifest::path(slice_dir);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(SnapshotId::from_digest(&sha256_hex(&bytes)))),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Filesystem {
            op: "read",
            path,
            source,
        }),
    }
}

/// Canonical empty digest for `profile` and `observations`: the
/// SHA-256 of zero bytes, until RFC-88 profiles / RFC-97 observations
/// define their own canonical encodings.
#[must_use]
pub fn empty_digest() -> SnapshotId {
    SnapshotId::from_digest(&sha256_hex(b""))
}

/// Content digest of one bundle file's raw bytes. `None` when the file
/// is absent. Shared with the `slice` crate's manifest assembly.
///
/// # Errors
///
/// Filesystem read failures other than absence.
pub fn content_digest(path: &Path) -> Result<Option<SnapshotId>, Error> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(SnapshotId::from_digest(&sha256_hex(&bytes)))),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source,
        }),
    }
}
