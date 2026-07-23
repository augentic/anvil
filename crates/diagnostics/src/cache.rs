//! Pure path and digest helpers for the out-of-tree artifact
//! locations.
//!
//! The adapter store and per-project component cache *roots* are
//! deployment configuration, resolved once at each composition root
//! and carried as a value (`project::handler::Locations`). This module
//! keeps only the root-parameterized math every deployment agrees on:
//! the stable per-project identifier, file content digesting, and the
//! verify-on-read digest sidecar shape. Nothing here reads the
//! environment or branches on the compilation target.
//!
//! Lives on the `diagnostics` leaf so every consumer resolves the same
//! shapes without a cross-layer dependency.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::digest::{Hasher, sha256_hex};

/// Stable per-project identifier — the SHA-256 hex of the canonicalised
/// project path, falling back to the raw path when canonicalisation
/// fails (e.g. the directory does not yet exist).
///
/// Keys the per-project cache directory beneath a host cache parent,
/// so the location is stable across invocations and unique per
/// checkout.
#[must_use]
pub fn project_id(project_dir: &Path) -> String {
    let canonical =
        std::fs::canonicalize(project_dir).unwrap_or_else(|_| project_dir.to_path_buf());
    sha256_hex(canonical.as_os_str().as_encoded_bytes())
}

/// Deterministic content digest of one file, in the `sha256:<hex>`
/// form. A store entry is a single component file, so the entry digest
/// is the file's byte digest.
///
/// Infallible by design, mirroring the other helpers — an unreadable
/// file digests as empty rather than poisoning the caller, since a
/// healthy read-only store entry never trips that path.
#[must_use]
pub fn file_content_digest(file: &Path) -> String {
    let bytes = std::fs::read(file).unwrap_or_default();
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    format!("sha256:{}", hasher.finalize_hex())
}

/// Verify-on-read sidecar contents. Registry-internal YAML;
/// deliberately *not* an embedded JSON Schema artifact.
#[derive(Debug, Serialize, Deserialize)]
struct StoreMeta {
    /// Deterministic [`file_content_digest`] of the installed component.
    tree_digest: String,
    /// Registry provenance recorded at install time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    oci: Option<OciProvenance>,
}

/// The OCI registry provenance recorded on every installed store entry.
///
/// The durable link between the local bytes and what the registry
/// served — the prerequisite for later tag-drift detection and
/// per-project digest pinning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OciProvenance {
    /// Registry repository the component was pulled from
    /// (`ghcr.io/<org>/<repo>/<name>`).
    pub repository: String,
    /// Resolved OCI manifest digest (`sha256:<hex>`).
    pub manifest_digest: String,
    /// Component layer content digest from the manifest descriptor
    /// (`sha256:<hex>`).
    pub layer_digest: String,
}

/// The recorded vs recomputed entry digests when verify-on-read fails.
///
/// Carried by [`StoreVerifyError::Mismatch`] when a store entry's
/// current tree content digest no longer matches the digest recorded
/// at install time — the signal that an immutable artifact has drifted
/// (a moved tag, a corrupted store entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestMismatch {
    /// Digest recorded in the sidecar at install time.
    pub recorded: String,
    /// Digest recomputed from the entry's current contents.
    pub actual: String,
}

/// Why [`verify_store_entry`] refused a store entry.
///
/// Fail-closed on both arms: an entry without a readable sidecar is as
/// unverifiable as one whose digest drifted — the resolver is the last
/// gate before the runtime executes the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreVerifyError {
    /// No sidecar exists (or it cannot be parsed) beside the entry.
    MissingSidecar,
    /// The recorded and recomputed digests differ.
    Mismatch(DigestMismatch),
}

/// Write the verify-on-read sidecar at `meta_path`, at install time.
///
/// `tree_digest` is the [`file_content_digest`] of the freshly
/// installed component; `oci` is the registry provenance, recorded for
/// every registry install (absent only for entries staged outside the
/// pull path, e.g. test fixtures). The sidecar is a writable sibling
/// of the read-only entry (`<name>@<version>.meta`).
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] when the sidecar cannot be
/// serialised or written.
pub fn write_store_meta(
    meta_path: &Path, tree_digest: &str, oci: Option<&OciProvenance>,
) -> std::io::Result<()> {
    let meta = StoreMeta {
        tree_digest: tree_digest.to_string(),
        oci: oci.cloned(),
    };
    let body =
        serde_saphyr::to_string(&meta).map_err(|err| std::io::Error::other(err.to_string()))?;
    std::fs::write(meta_path, body)
}

/// Read the recorded tree digest from the verify-on-read sidecar at
/// `meta_path`, or `None` when no sidecar exists or it cannot be
/// parsed.
#[must_use]
pub fn read_store_meta(meta_path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(meta_path).ok()?;
    let meta: StoreMeta = serde_saphyr::from_str(&raw).ok()?;
    Some(meta.tree_digest)
}

/// Read the recorded OCI provenance from the sidecar at `meta_path`,
/// when present.
#[must_use]
pub fn read_store_provenance(meta_path: &Path) -> Option<OciProvenance> {
    let raw = std::fs::read_to_string(meta_path).ok()?;
    let meta: StoreMeta = serde_saphyr::from_str(&raw).ok()?;
    meta.oci
}

/// Verify a store entry against its recorded digest (verify-on-read).
///
/// Reads the recorded digest from the sidecar at `meta_path` and
/// recomputes [`file_content_digest`] over the component file at
/// `entry`. Fail-closed: a missing or unparseable sidecar refuses the
/// entry ([`StoreVerifyError::MissingSidecar`]) — every install writes
/// a sidecar, so an entry without one is unverifiable, not legacy.
///
/// # Errors
///
/// [`StoreVerifyError::MissingSidecar`] when no sidecar can be read;
/// [`StoreVerifyError::Mismatch`] when the recorded and recomputed
/// digests differ.
pub fn verify_store_entry(entry: &Path, meta_path: &Path) -> Result<(), StoreVerifyError> {
    let Some(recorded) = read_store_meta(meta_path) else {
        return Err(StoreVerifyError::MissingSidecar);
    };
    let actual = file_content_digest(entry);
    if actual == recorded {
        Ok(())
    } else {
        Err(StoreVerifyError::Mismatch(DigestMismatch { recorded, actual }))
    }
}
