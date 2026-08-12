//! Content-addressed per-slice build records.
//!
//! Build outcomes live at `.emery/slices/<slice>/builds/<digest>.yaml`;
//! "built" projects from these records and facts, never from a path check.

use std::path::{Path, PathBuf};

use diagnostics::digest::sha256_hex;
use error::Error;
use serde::{Deserialize, Serialize};

use crate::config::Layout;
use crate::seam::wire::BuildReport;
use crate::snapshot::{CodePatch, SnapshotId};

/// On-disk fact-substrate build record (RFC-86 D27).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildRecord {
    /// Snapshot the build prepared from — the wave-open base
    /// (RFC-91 D6).
    pub base: SnapshotId,
    /// Snapshot captured from the private workspace result tree.
    pub result: SnapshotId,
    /// Sorted workspace-relative paths that differ between the trees.
    pub touched: Vec<String>,
    /// Content digest of the one-member wave that authorized this build.
    pub wave: SnapshotId,
    /// Typed build report validated by the finalize tail.
    pub report: BuildReport,
    /// Sorted requirement-body digests of the deferred set this build
    /// consumed (RFC-86a D4) — the input fence for disposition drift:
    /// a built slice whose live disposition set no longer matches is
    /// stale, exactly like pin drift. Empty when nothing was deferred.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deferred: Vec<String>,
}

/// Result of persisting a content-addressed build record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// Content digest of the written YAML (`sha256:…`).
    pub digest: SnapshotId,
    /// Absolute path of the written file.
    pub path: PathBuf,
}

impl BuildRecord {
    /// Assemble a record from the captured patch, open wave, report,
    /// and the consumed deferred digest set (sorted and deduplicated
    /// on entry — identical requirement bodies legally share one
    /// digest, RFC-86a D2).
    #[must_use]
    pub fn from_capture(
        patch: CodePatch, wave: SnapshotId, report: BuildReport, mut deferred: Vec<String>,
    ) -> Self {
        deferred.sort_unstable();
        deferred.dedup();
        Self {
            base: patch.base,
            result: patch.result,
            touched: patch.touched,
            wave,
            report,
            deferred,
        }
    }

    /// Project the RFC-87 [`CodePatch`] shape merge still consumes.
    #[must_use]
    pub fn to_patch(&self) -> CodePatch {
        CodePatch {
            base: self.base.clone(),
            result: self.result.clone(),
            touched: self.touched.clone(),
        }
    }

    /// Canonical YAML bytes (trailing newline).
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn canonical_yaml(&self) -> Result<String, Error> {
        artifacts::atomic::serialise_yaml(self)
    }

    /// Content digest of [`Self::canonical_yaml`].
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        Ok(SnapshotId::from_digest(&sha256_hex(self.canonical_yaml()?.as_bytes())))
    }

    /// Whether `slice_dir` holds at least one build record.
    #[must_use]
    pub fn present(slice_dir: &Path) -> bool {
        let dir = builds_dir(slice_dir);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return false;
        };
        entries.filter_map(Result::ok).any(|entry| {
            entry.path().extension().and_then(|ext| ext.to_str()) == Some("yaml")
                && entry.path().is_file()
        })
    }

    /// Atomically write this record at its content-addressed path under
    /// `slice_dir/builds/`.
    ///
    /// Write-once: identical bytes at the digest path are a no-op;
    /// divergent bytes are `slice-build-record-conflict`.
    ///
    /// # Errors
    ///
    /// YAML / filesystem failures; digest collision.
    pub fn write(&self, slice_dir: &Path) -> Result<Written, Error> {
        let yaml = self.canonical_yaml()?;
        let digest = SnapshotId::from_digest(&sha256_hex(yaml.as_bytes()));
        let path = record_path(slice_dir, digest.digest());
        if path.is_file() {
            let existing = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
                op: "read",
                path: path.clone(),
                source,
            })?;
            if existing == yaml {
                return Ok(Written { digest, path });
            }
            return Err(Error::Diag {
                code: "slice-build-record-conflict",
                detail: format!(
                    "build record at `{}` already exists with different bytes",
                    path.display()
                ),
            });
        }
        artifacts::atomic::bytes_write(&path, yaml.as_bytes())?;
        Ok(Written { digest, path })
    }

    /// Load the build record authorized by `wave` under
    /// `slice_dir/builds/`.
    ///
    /// Selection is by the recorded wave digest, never file mtime, so
    /// a stale record (earlier epoch, restored tree with scrambled
    /// timestamps) cannot shadow the authorized build.
    ///
    /// # Errors
    ///
    /// `slice-build-record-missing` when no record names `wave`;
    /// `slice-build-record-ambiguous` when more than one does; parse /
    /// IO failures on present records.
    pub fn load_for_wave(slice_dir: &Path, wave: &SnapshotId) -> Result<Self, Error> {
        Self::find_for_wave(slice_dir, wave)?.ok_or_else(|| missing_for_wave(slice_dir, wave))
    }

    /// The unique build record naming `wave`, or `None` when no record
    /// does — the probe form of [`Self::load_for_wave`] for callers
    /// (disposition-drift staleness) that treat a missing record as a
    /// state, not a refusal.
    ///
    /// # Errors
    ///
    /// `slice-build-record-ambiguous` when more than one record names
    /// `wave`; parse / IO failures on present records.
    pub fn find_for_wave(slice_dir: &Path, wave: &SnapshotId) -> Result<Option<Self>, Error> {
        let mut matches: Vec<Self> =
            Self::load_all(slice_dir)?.into_iter().filter(|record| record.wave == *wave).collect();
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.remove(0))),
            found => Err(Error::Diag {
                code: "slice-build-record-ambiguous",
                detail: format!(
                    "slice `{}` has {found} build records naming wave `{wave}`; remove the \
                     stale `builds/<digest>.yaml` duplicates before merging",
                    slice_name(slice_dir)
                ),
            }),
        }
    }

    /// Load the newest build record under `slice_dir/builds/` (by
    /// modification time; ties break on path).
    ///
    /// Inspection helper for labs and tests; merge authority resolves
    /// the record by its wave fact via [`Self::load_for_wave`].
    ///
    /// # Errors
    ///
    /// `slice-build-record-missing` when none exist; parse / IO failures.
    pub fn load_latest(slice_dir: &Path) -> Result<Self, Error> {
        let dir = builds_dir(slice_dir);
        let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(missing(slice_dir));
            }
            Err(source) => {
                return Err(Error::Filesystem {
                    op: "read_dir",
                    path: dir,
                    source,
                });
            }
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") || !path.is_file() {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let replace = match &newest {
                Some((stamp, prior)) => modified > *stamp || (modified == *stamp && path > *prior),
                None => true,
            };
            if replace {
                newest = Some((modified, path));
            }
        }
        let Some((_, path)) = newest else {
            return Err(missing(slice_dir));
        };
        load_path(&path)
    }

    /// Load every build record under `slice_dir/builds/` (any order).
    /// An absent directory is empty, not an error.
    ///
    /// # Errors
    ///
    /// Parse / IO failures on present records.
    pub fn load_all(slice_dir: &Path) -> Result<Vec<Self>, Error> {
        let dir = builds_dir(slice_dir);
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(source) => {
                return Err(Error::Filesystem {
                    op: "read_dir",
                    path: dir,
                    source,
                });
            }
        };
        let mut records = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") && path.is_file() {
                records.push(load_path(&path)?);
            }
        }
        Ok(records)
    }

    /// Persist via [`Layout`] helpers (same bytes as [`Self::write`]).
    ///
    /// # Errors
    ///
    /// Propagates [`Self::write`].
    pub fn write_for(&self, layout: Layout<'_>, slice: &str) -> Result<Written, Error> {
        self.write(&layout.slice_dir(slice))
    }
}

fn builds_dir(slice_dir: &Path) -> PathBuf {
    slice_dir.join("builds")
}

fn record_path(slice_dir: &Path, digest: &str) -> PathBuf {
    builds_dir(slice_dir).join(format!("{digest}.yaml"))
}

fn load_path(path: &Path) -> Result<BuildRecord, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
        op: "read",
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_saphyr::from_str(&text)?)
}

fn slice_name(slice_dir: &Path) -> &str {
    slice_dir.file_name().and_then(|s| s.to_str()).unwrap_or("unknown")
}

fn missing(slice_dir: &Path) -> Error {
    Error::validation_failed(
        "slice-build-record-missing",
        "a built slice carries a fact-substrate build record",
        format!(
            "slice `{}` has no `builds/<digest>.yaml`; re-run `emery plan execute` so the \
             build phase records it before merging",
            slice_name(slice_dir)
        ),
    )
}

fn missing_for_wave(slice_dir: &Path, wave: &SnapshotId) -> Error {
    Error::validation_failed(
        "slice-build-record-missing",
        "the merge phase loads the build record its authorized wave names",
        format!(
            "slice `{}` has no `builds/<digest>.yaml` recording wave `{wave}`; re-run \
             `emery plan execute` so the build phase records it before merging",
            slice_name(slice_dir)
        ),
    )
}
