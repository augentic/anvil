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
    /// Registry content digest recorded for provenance only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    layer_digest: Option<String>,
}

/// The recorded vs recomputed entry digests when verify-on-read fails.
///
/// Returned by [`verify_store_entry`] when a store entry's current tree
/// content digest no longer matches the digest recorded at install time
/// — the signal that an immutable artifact has drifted (a moved tag, a
/// corrupted store entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestMismatch {
    /// Digest recorded in the sidecar at install time.
    pub recorded: String,
    /// Digest recomputed from the entry's current contents.
    pub actual: String,
}

/// Write the verify-on-read sidecar at `meta_path`, at install time.
///
/// `tree_digest` is the [`file_content_digest`] of the freshly
/// installed component; `layer_digest` is the registry content digest,
/// recorded for provenance when known. The sidecar is a writable
/// sibling of the read-only entry (`<name>@<version>.meta`).
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] when the sidecar cannot be
/// serialised or written.
pub fn write_store_meta(
    meta_path: &Path, tree_digest: &str, layer_digest: Option<&str>,
) -> std::io::Result<()> {
    let meta = StoreMeta {
        tree_digest: tree_digest.to_string(),
        layer_digest: layer_digest.map(ToString::to_string),
    };
    let body =
        serde_saphyr::to_string(&meta).map_err(|err| std::io::Error::other(err.to_string()))?;
    std::fs::write(meta_path, body)
}

/// Read the recorded tree digest from the verify-on-read sidecar at
/// `meta_path`, or `None` when no sidecar exists or it cannot be
/// parsed.
///
/// `None` is the fail-open signal for a legacy or foreign store entry
/// installed before the sidecar existed — verify-on-read treats it as a
/// pass rather than refusing the entry.
#[must_use]
pub fn read_store_meta(meta_path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(meta_path).ok()?;
    let meta: StoreMeta = serde_saphyr::from_str(&raw).ok()?;
    Some(meta.tree_digest)
}

/// Verify a store entry against its recorded digest (verify-on-read).
///
/// Reads the recorded digest from the sidecar at `meta_path`,
/// recomputes [`file_content_digest`] over the component file at
/// `entry`, and reports a [`DigestMismatch`] when they differ. A
/// missing sidecar is fail-open (`Ok`): legacy and foreign entries
/// predate the sidecar, and the entry's own read-only immutability
/// remains the baseline guarantee.
///
/// # Errors
///
/// Returns [`DigestMismatch`] when the recorded and recomputed digests
/// differ.
pub fn verify_store_entry(entry: &Path, meta_path: &Path) -> Result<(), DigestMismatch> {
    let Some(recorded) = read_store_meta(meta_path) else {
        return Ok(());
    };
    let actual = file_content_digest(entry);
    if actual == recorded { Ok(()) } else { Err(DigestMismatch { recorded, actual }) }
}
