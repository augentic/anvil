//! YAML sidecar for init-time AGENTS.md generation fingerprints.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use artifacts::atomic::yaml_write;
use error::Error;
use serde::{Deserialize, Serialize};

use super::fingerprint::ContextFingerprint;

const CURRENT_LOCK_VERSION: u64 = 1;

/// The `.specify/context.lock` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Repo-relative path, `/`-separated.
    pub path: String,
    /// Lowercase hex SHA-256 digest of the input bytes.
    pub sha256: String,
}

/// Fence digests on [`ContextLock::fences`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fences {
    /// `sha256:<hex>` digest of the bytes between the context fences.
    pub body_sha256: String,
}

#[derive(Debug, Deserialize)]
struct Version {
    version: u64,
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

/// Read the lock at `path`; `Ok(None)` when the file does not exist.
///
/// # Errors
///
/// [`Error::Io`] on read failures other than not-found, and
/// [`Error::Validation`] (`context-lock-malformed` /
/// `context-lock-version-too-new`) when the document does not parse or
/// carries an unsupported version.
pub fn load(path: &Path) -> Result<Option<ContextLock>, Error> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(Error::Io(err)),
    };

    let version: Version = serde_saphyr::from_str(&contents).map_err(|err| {
        validation_error(
            "context-lock-malformed",
            format!("context-lock-malformed: failed to read lock version: {err}"),
        )
    })?;
    if version.version > CURRENT_LOCK_VERSION {
        return Err(validation_error(
            "context-lock-version-too-new",
            format!(
                "context-lock-version-too-new: lock version {} > supported {CURRENT_LOCK_VERSION}",
                version.version
            ),
        ));
    }
    if version.version != CURRENT_LOCK_VERSION {
        return Err(validation_error(
            "context-lock-malformed",
            format!(
                "context-lock-malformed: unsupported lock version {}; expected \
                 {CURRENT_LOCK_VERSION}",
                version.version
            ),
        ));
    }

    let lock: ContextLock = serde_saphyr::from_str(&contents).map_err(|err| {
        validation_error("context-lock-malformed", format!("context-lock-malformed: {err}"))
    })?;
    Ok(Some(lock))
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

fn validation_error(rule_id: &'static str, detail: String) -> Error {
    Error::validation_failed(rule_id, "context.lock must be a supported context lock file", detail)
}

// The `load` / `save` codec — cold-start `None`, the round-trip,
// snake_case serialisation, and the version gate's failure shapes — is
// exercised through the public API in `crates/workflow/tests/agents_lock.rs`.
