//! YAML sidecar for init-time AGENTS.md generation fingerprints.

use std::path::Path;

use artifacts::atomic::yaml_write;
use error::Error;
use serde::Serialize;

use super::fingerprint::ContextFingerprint;

const CURRENT_LOCK_VERSION: u64 = 1;

/// The `.specify/context.lock` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextLock {
    /// Lock format version; currently always `1`.
    pub version: u64,
    /// `sha256:<hex>` digest over the canonical aggregate input.
    pub fingerprint: String,
    /// CLI version that generated the fenced block.
    pub cli_version: String,
    /// Sorted per-file input digests.
    pub inputs: Vec<Input>,
    /// Digests of the generated fenced block itself.
    pub fences: Fences,
}

/// One fingerprinted renderer input on [`ContextLock::inputs`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Repo-relative path, `/`-separated.
    pub path: String,
    /// Lowercase hex SHA-256 digest of the input bytes.
    pub sha256: String,
}

/// Fence digests on [`ContextLock::fences`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fences {
    /// `sha256:<hex>` digest of the bytes between the context fences.
    pub body_sha256: String,
}

impl ContextLock {
    // YAML sidecar persisted to disk — not a wire DTO, so it keeps a
    // named constructor rather than a `From`-for-Body/Row impl.
    /// Build the current-version lock from a computed fingerprint.
    #[must_use]
    pub fn from_fingerprint(fingerprint: &ContextFingerprint) -> Self {
        Self {
            version: CURRENT_LOCK_VERSION,
            fingerprint: fingerprint.fingerprint.clone(),
            cli_version: fingerprint.cli_version.clone(),
            inputs: fingerprint
                .inputs
                .iter()
                .map(|input| Input {
                    path: input.path.clone(),
                    sha256: input.sha256.clone(),
                })
                .collect(),
            fences: Fences {
                body_sha256: fingerprint.body_sha256.clone(),
            },
        }
    }
}

/// Atomically write the lock to `path`.
///
/// # Errors
///
/// Propagates the atomic-write failure.
pub fn save(path: &Path, lock: &ContextLock) -> Result<(), Error> {
    // ContextLock isn't a Plan/Registry/ProjectConfig sibling; its load
    // path returns a typed Validation envelope rather than `Option<Self>`,
    // so it doesn't fit the AtomicYaml shape.
    yaml_write(path, lock)
}
