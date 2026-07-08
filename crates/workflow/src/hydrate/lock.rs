//! `.specify/adapters.lock` — the committed cross-machine digest pin
//! (§"Cloud posture").
//!
//! Maps each hydrated identity (`<name>@<version>`) to its
//! component-byte digest (`sha256:<hex>` — the same digest the store's
//! `.meta` sidecar records). Written by the hydration kernel at first
//! install and verified on every subsequent hydration, so a cloud
//! runner's install is byte-equivalent to the laptop that authored the
//! pin. Committed like any lockfile; machine-written, never
//! hand-edited.
//!
//! Entries for identities no longer declared are left in place —
//! store entries are immutable and shared across projects, so pruning
//! is a non-goal here.

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use artifacts::atomic::yaml_write;
use error::Error;
use serde::{Deserialize, Serialize};

/// Current `adapters.lock` schema version.
pub const CURRENT_ADAPTERS_LOCK_VERSION: u64 = 1;

/// In-memory representation of `.specify/adapters.lock`.
///
/// The `BTreeMap` keeps keys sorted, so serialization is deterministic
/// and diff-friendly by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdaptersLock {
    /// Schema version. `1` is the only accepted value for this release.
    pub version: u64,
    /// `<name>@<version>` → `sha256:<hex>` component-byte digest.
    #[serde(default)]
    pub adapters: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Version {
    version: u64,
}

impl Default for AdaptersLock {
    fn default() -> Self {
        Self {
            version: CURRENT_ADAPTERS_LOCK_VERSION,
            adapters: BTreeMap::new(),
        }
    }
}

impl AdaptersLock {
    /// Load + version-gate the committed lock. A missing file yields
    /// `Ok(None)` — first hydration creates it.
    ///
    /// # Errors
    ///
    /// - [`Error::Validation`] `adapters-lock-malformed` when the YAML
    ///   does not parse or carries an unsupported version.
    /// - [`Error::Validation`] `adapters-lock-version-too-new` when the
    ///   version is newer than this binary supports.
    pub fn load(path: &Path) -> Result<Option<Self>, Error> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(Error::Io(err)),
        };

        let version: Version = serde_saphyr::from_str(&contents).map_err(|err| {
            malformed(format!("adapters-lock-malformed: failed to read lock version: {err}"))
        })?;
        if version.version > CURRENT_ADAPTERS_LOCK_VERSION {
            return Err(Error::validation_failed(
                "adapters-lock-version-too-new",
                ".specify/adapters.lock must be a supported version",
                format!(
                    "adapters-lock-version-too-new: lock version {} > supported \
                     {CURRENT_ADAPTERS_LOCK_VERSION}",
                    version.version
                ),
            ));
        }
        if version.version != CURRENT_ADAPTERS_LOCK_VERSION {
            return Err(malformed(format!(
                "adapters-lock-malformed: unsupported lock version {}; expected \
                 {CURRENT_ADAPTERS_LOCK_VERSION}",
                version.version
            )));
        }

        let lock: Self = serde_saphyr::from_str(&contents)
            .map_err(|err| malformed(format!("adapters-lock-malformed: {err}")))?;
        Ok(Some(lock))
    }

    /// Atomically write the lock (sorted keys, trailing newline).
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error on a failed write.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        yaml_write(path, self)
    }
}

fn malformed(detail: String) -> Error {
    Error::validation_failed(
        "adapters-lock-malformed",
        ".specify/adapters.lock must be a supported adapters lock file",
        detail,
    )
}
