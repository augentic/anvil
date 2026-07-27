//! Slice-directory creation for the refine orchestration.

use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use project::name::validate_name;
use serde::Serialize;

use crate::{LifecycleStatus, SliceMetadata};

/// What to do when slice creation finds an existing directory at the
/// target path. Rides the wire typed on both transports (kebab-case
/// values), matching the CLI mirror's value parser.
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    strum::VariantNames,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum CreateIfExists {
    /// Refuse when the directory exists (default).
    #[default]
    Fail,
    /// Reuse the existing directory — requires a valid `metadata.yaml`.
    /// Intended for the define skill's "continue in-flight slice" flow.
    Continue,
    /// Delete and recreate — destructive. Intended for the define
    /// skill's "restart" flow. The caller is expected to have already
    /// archived anything it wants to keep.
    Restart,
}

/// Outcome of [`create`], surfacing whether a new directory was written
/// or an existing one was reused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[must_use]
pub struct Created {
    /// Path to the slice directory.
    #[serde(serialize_with = "ser_dir")]
    pub dir: PathBuf,
    /// Loaded or freshly-created metadata.
    #[serde(flatten)]
    pub metadata: SliceMetadata,
    /// `true` when the call created a new directory; `false` when an
    /// existing directory was reused (`CreateIfExists::Continue`).
    pub is_new: bool,
    /// `true` when the call replaced an existing directory
    /// (`CreateIfExists::Restart`).
    pub restarted: bool,
}

#[expect(clippy::ptr_arg, reason = "serde `serialize_with` requires `&PathBuf`")]
fn ser_dir<S: serde::Serializer>(v: &PathBuf, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(&v.display())
}

/// Create `<parent_dir>/<name>/` and seed an initial `metadata.yaml`.
///
/// - `parent_dir` is expected to be `<project>/.emery/slices/`.
/// - `now` is plumbed in so tests can pin `created_at` deterministically.
///
/// On success returns a [`Created`] with the resolved directory and
/// loaded metadata. Behaviour when the directory already exists is
/// governed by `if_exists` — see [`CreateIfExists`].
///
/// # Errors
///
/// `Error::Diag` with `code = "invalid-name"` for a non-kebab `name`;
/// `Error::Diag` with `slice-already-exists` / `slice-dir-missing-metadata`
/// for the existing-dir branches; otherwise propagates I/O or save failures.
pub fn create(
    parent_dir: &Path, name: &str, target: &str, if_exists: CreateIfExists, now: Timestamp,
) -> Result<Created, Error> {
    validate_name(name)?;
    let slice_dir = parent_dir.join(name);
    let metadata_path = SliceMetadata::path(&slice_dir);

    if slice_dir.exists() {
        match if_exists {
            CreateIfExists::Fail => {
                return Err(Error::Diag {
                    code: "slice-already-exists",
                    detail: format!("slice `{name}` already exists at {}", slice_dir.display()),
                });
            }
            CreateIfExists::Continue => {
                if !metadata_path.exists() {
                    return Err(Error::Diag {
                        code: "slice-dir-missing-metadata",
                        detail: format!(
                            "slice dir {} exists but has no metadata.yaml; refusing to reuse",
                            slice_dir.display()
                        ),
                    });
                }
                let metadata = SliceMetadata::load(&slice_dir)?;
                return Ok(Created {
                    dir: slice_dir,
                    metadata,
                    is_new: false,
                    restarted: false,
                });
            }
            CreateIfExists::Restart => {
                std::fs::remove_dir_all(&slice_dir)?;
            }
        }
    }

    std::fs::create_dir_all(slice_dir.join("specs"))?;
    let metadata = SliceMetadata {
        target: target.to_string(),
        status: LifecycleStatus::Refining,
        created_at: Some(now),
        defined_at: None,
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: Vec::new(),
        outcome: None,
    };
    metadata.save(&slice_dir)?;

    Ok(Created {
        dir: slice_dir,
        metadata,
        is_new: true,
        restarted: matches!(if_exists, CreateIfExists::Restart),
    })
}
