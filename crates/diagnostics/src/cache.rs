//! Digest math and the verify-on-read sidecar format for the engine's
//! adapter store.
//!
//! Only value-in/value-out helpers live here — callers fetch entry
//! bytes and sidecar text through their storage capabilities; nothing
//! reads the environment or touches the filesystem.

use serde::{Deserialize, Serialize};

use crate::digest::Hasher;

/// The `sha256:<hex>` content digest of an in-memory payload. A store
/// entry is a single component, so the entry digest is its byte
/// digest.
#[must_use]
pub fn content_digest(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    format!("sha256:{}", hasher.finalize_hex())
}

// Verify-on-read sidecar contents. Registry-internal YAML;
// deliberately *not* an embedded JSON Schema artifact.
#[derive(Debug, Serialize, Deserialize)]
struct StoreMeta {
    // Deterministic [`content_digest`] of the installed component.
    tree_digest: String,
    // Registry provenance recorded at install time.
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
/// Carried when a store entry's current content digest no longer
/// matches the digest recorded at install time — the signal that an
/// immutable artifact has drifted (a moved tag, a corrupted store
/// entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestMismatch {
    /// Digest recorded in the sidecar at install time.
    pub recorded: String,
    /// Digest recomputed from the entry's current contents.
    pub actual: String,
}

/// Parse the recorded tree digest out of sidecar text, fetched by the
/// caller through its storage capability. `None` when the text is not
/// a sidecar.
#[must_use]
pub fn recorded_digest(sidecar: &str) -> Option<String> {
    let meta: StoreMeta = serde_saphyr::from_str(sidecar).ok()?;
    Some(meta.tree_digest)
}

/// Parse the recorded OCI provenance out of sidecar text, when
/// present.
#[must_use]
pub fn recorded_provenance(sidecar: &str) -> Option<OciProvenance> {
    let meta: StoreMeta = serde_saphyr::from_str(sidecar).ok()?;
    meta.oci
}
