//! Adapter-store digests and verify-on-read sidecars.
//!
//! Only value-in/value-out helpers live here — callers fetch entry
//! bytes and sidecar text through their storage capabilities; nothing
//! reads the environment or touches the filesystem.

use serde::{Deserialize, Serialize};

use crate::digest::Hasher;

/// Return an in-memory payload's `sha256:<hex>` digest.
#[must_use]
pub fn content_digest(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    format!("sha256:{}", hasher.finalize_hex())
}

// Registry-internal YAML, not an embedded schema artifact.
#[derive(Debug, Serialize, Deserialize)]
struct StoreMeta {
    tree_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    oci: Option<OciProvenance>,
}

/// OCI provenance for an installed store entry.
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

/// Recorded and recomputed digests for verify-on-read failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestMismatch {
    /// Digest recorded in the sidecar at install time.
    pub recorded: String,
    /// Digest recomputed from the entry's current contents.
    pub actual: String,
}

/// Parse a sidecar's tree digest, or `None` for invalid text.
#[must_use]
pub fn recorded_digest(sidecar: &str) -> Option<String> {
    let meta: StoreMeta = serde_saphyr::from_str(sidecar).ok()?;
    Some(meta.tree_digest)
}

/// Parse a sidecar's optional OCI provenance.
#[must_use]
pub fn recorded_provenance(sidecar: &str) -> Option<OciProvenance> {
    let meta: StoreMeta = serde_saphyr::from_str(sidecar).ok()?;
    meta.oci
}
