//! Disk I/O for `plan.yaml`: atomic load and save.
//!
//! Filesystem moves for archival live in [`super::archive`].

use std::path::{Path, PathBuf};

use specify_error::Error;
use specify_model::atomic::yaml_write;

use super::model::Plan;
use crate::config::{AtomicYaml, Layout};
use crate::schema::validate_plan_yaml;

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
    /// Load `plan.yaml` (at the repo root) from disk.
    ///
    /// Errors mirror [`crate::slice::SliceMetadata::load`]:
    ///   - missing file -> `Error::ArtifactNotFound`
    ///   - schema failure -> `Error::Validation`
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
        validate_plan_yaml(&content)?;
        let plan: Self = serde_saphyr::from_str(&content)?;
        Ok(plan)
    }

    /// Serialize and write the plan to `path`, overwriting if present.
    ///
    /// Atomic: a partial file is never observed by readers. Write goes via
    /// a temp file in the same directory followed by `fs::rename`. Because
    /// POSIX `rename(2)` (and Windows `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING`)
    /// are atomic at the filesystem level, any concurrent reader of `path`
    /// sees either the previous complete contents or the new complete
    /// contents — never a half-written or empty file.
    ///
    /// Always emits a trailing newline so the on-disk form matches the
    /// convention used elsewhere in the project and so POSIX text-file
    /// tools (`wc -l`, `sed`, `grep`) behave predictably.
    ///
    /// # Errors
    ///
    /// Returns `Error::Io` on any I/O failure and `Error::YamlSer` if
    /// serialization fails.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        yaml_write(path, self)
    }
}

#[cfg(test)]
mod tests;
