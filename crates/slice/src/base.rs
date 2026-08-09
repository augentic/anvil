//! Refine-time pin assembly (`base.yaml`).
//!
//! Build prepares from the pin refine recorded — never an ambient
//! freeze at build start; validate reports stale pins as drift.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use error::Error;
use project::plan::{Entry, Plan, dir_cid};
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};

/// On-disk `.emery/slices/<slice>/base.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Base {
    /// Per-source tree `cid`s copied from the closed plan source set
    /// for every binding on this slice.
    pub sources: BTreeMap<String, SnapshotId>,
    /// Content-addressed identity of the baseline `specs/` tree.
    pub baseline_spec: SnapshotId,
    /// Product-tree (target-base) snapshot identity recorded at refine
    /// via [`project::seam::Workspaces::freeze`]. Build reads this pin
    /// into `prepare` (RFC-86 D25 / D27).
    pub target_base: SnapshotId,
}

impl Base {
    /// Path of the pin assembly file under `slice_dir`.
    #[must_use]
    pub fn path(slice_dir: &Path) -> PathBuf {
        slice_dir.join("base.yaml")
    }

    /// Assemble pins from the plan's closed source set, the
    /// baseline-spec digest at `baseline_specs_dir`, and a caller-
    /// frozen target-base snapshot.
    ///
    /// # Errors
    ///
    /// `slice-base-source-unbound` when an entry binding names a key
    /// absent from `plan.sources`; `slice-base-pin-missing` when a
    /// bound source has no `cid` yet; filesystem failures from the
    /// baseline tree digest.
    pub fn assemble(
        plan: &Plan, entry: &Entry, baseline_specs_dir: &Path, target_base: SnapshotId,
    ) -> Result<Self, Error> {
        let mut sources = BTreeMap::new();
        for binding in &entry.sources {
            let key = binding.source();
            let Some(bound) = plan.sources.get(key) else {
                return Err(Error::Diag {
                    code: "slice-base-source-unbound",
                    detail: format!(
                        "slice `{}` binds source `{key}` which is absent from plan.yaml.sources",
                        entry.name
                    ),
                });
            };
            let Some(cid) = bound.cid.clone() else {
                return Err(Error::Diag {
                    code: "slice-base-pin-missing",
                    detail: format!(
                        "source `{key}` has no cid pin; re-run `emery plan author` to close \
                         the source set"
                    ),
                });
            };
            sources.insert(key.to_string(), cid);
        }
        Ok(Self {
            sources,
            baseline_spec: dir_cid(baseline_specs_dir)?,
            target_base,
        })
    }

    /// Atomically write this assembly to [`Self::path`].
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization and filesystem failures.
    pub fn write(&self, slice_dir: &Path) -> Result<(), Error> {
        artifacts::atomic::yaml_write(&Self::path(slice_dir), self)
    }

    /// Load a previously written assembly.
    ///
    /// # Errors
    ///
    /// Filesystem / YAML parse failures.
    pub fn load(slice_dir: &Path) -> Result<Self, Error> {
        let path = Self::path(slice_dir);
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        Ok(serde_saphyr::from_str(&text)?)
    }
}
