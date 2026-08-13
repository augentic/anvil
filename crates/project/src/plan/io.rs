//! Disk I/O for `plan.yaml`: atomic load and save.
//!
//! Filesystem moves for archival live in [`super::archive`].

use std::path::{Path, PathBuf};

use artifacts::atomic::yaml_write;
use error::Error;

use super::model::Plan;
use crate::config::{AtomicYaml, Layout};

impl AtomicYaml for Plan {
    fn layout_path(layout: Layout<'_>) -> PathBuf {
        layout.plan_path()
    }

    /// Trait-side loader: `Ok(None)` when the file is absent, mirroring
    /// the contract documented on [`AtomicYaml::load_state`]. Disambiguated
    /// from the inherent [`Plan::load`] (which returns
    /// `Error::ArtifactNotFound` on absence) so the trait helper can
    /// branch on `None` without inspecting the error variant.
    fn load_state(layout: Layout<'_>) -> Result<Option<Self>, Error> {
        let path = Self::layout_path(layout);
        if !path.exists() {
            return Ok(None);
        }
        Self::load(&path).map(Some)
    }
}

impl Plan {
    /// Load `plan.yaml` from disk.
    ///
    /// Errors mirror [`crate::slice::SliceMetadata::load`]:
    ///   - missing file -> `Error::ArtifactNotFound`
    ///   - YAML/type deserialization failure -> `Error::YamlDe`
    ///   - other I/O failure -> `Error::Io`
    ///
    /// Tolerant of files with or without a trailing newline —
    /// `serde_saphyr::from_str` accepts both.
    ///
    /// # Errors
    ///
    /// See variants enumerated above.
    pub fn load(path: &Path) -> Result<Self, Error> {
        if !path.exists() {
            return Err(Error::ArtifactNotFound {
                kind: "plan.yaml",
                path: path.to_path_buf(),
            });
        }
        let content = std::fs::read_to_string(path)?;
        let plan: Self = serde_saphyr::from_str(&content)?;
        Ok(plan)
    }

    /// `sha256:…` content digest of the on-disk `plan.yaml` bytes — the
    /// identity `plan.execute.started` coverage stamps and the pre-build
    /// gate re-checks.
    ///
    /// # Errors
    ///
    /// `Error::Io` when the file cannot be read.
    pub fn file_digest(layout: Layout<'_>) -> Result<String, Error> {
        let bytes = std::fs::read(layout.plan_path())?;
        Ok(format!("sha256:{}", diagnostics::digest::sha256_hex(&bytes)))
    }

    /// Serialize and write the plan to `path`, overwriting if present.
    ///
    /// Atomic: a temp file in the same directory then `rename`, so a
    /// concurrent reader sees either the old or the new complete
    /// contents — never a half-written file. Always emits a trailing
    /// newline.
    ///
    /// # Errors
    ///
    /// Returns `Error::Io` on any I/O failure and `Error::YamlSer` if
    /// serialization fails.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        yaml_write(path, self)
    }
}
