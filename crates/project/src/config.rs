//! `ProjectConfig` — in-memory model of `.emery/project.yaml` — and
//! `Layout<'a>`, the typed home for every `.emery/` path helper.
//!
//! [`Roots`] selects in-place vs detached change homes.

mod atomic;
mod roots;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use atomic::{AtomicYaml, Mutation, with_state};
use error::Error;
pub use roots::Roots;
use serde::{Deserialize, Serialize};

use crate::platform::Platform;

/// Directory name of the in-place change home under `.emery/`.
pub const CHANGE_DIR_NAME: &str = "change";

/// In-memory representation of `.emery/project.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProjectConfig {
    /// Project name (defaults to the project directory name at init time).
    pub name: String,

    /// Free-text description of the project's tech stack, architecture,
    /// and testing approach. Falls back to the adapter's domain when empty.
    ///
    /// Authored intent only. A project's *derived* routing identity —
    /// the `surface[]` of owned domains and a `recent[]` merge tail — is
    /// projected from its baseline (`.emery/specs/` + journal), never
    /// re-authored here. Unknown facets such as `capabilities` /
    /// `keywords` are silently ignored (this struct does not
    /// `deny_unknown_fields`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Adapter identifier — either a bare name (`omnia`) or a URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,

    /// Minimum `emery` CLI version required to operate on this project,
    /// serialised as the `emery:` key in `project.yaml`. Written by
    /// `emery init` as the running binary's version and enforced by
    /// [`ProjectConfig::load`] via the `semver` crate.
    #[serde(rename = "emery", default, skip_serializing_if = "Option::is_none")]
    pub emery_version: Option<String>,

    /// Map of artifact key (`proposal`, `specs`, `design`, `tasks`) to a
    /// path (relative to `.emery/`) of a markdown file containing extra
    /// rules for that artifact. Scaffolded with one empty entry per key
    /// by `emery init`.
    #[serde(default)]
    pub rules: BTreeMap<String, String>,

    /// Target platforms this project builds for (e.g. `core`, `ios`,
    /// `android`). Set at `emery init --platforms` and changeable via
    /// `emery init --upgrade --platforms`. When the bound target
    /// adapter declares `platforms.required`, this field must be
    /// non-empty and must include `Platform::Core`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<Platform>,
}
impl ProjectConfig {
    /// Load `.emery/project.yaml` from `project_dir`.
    ///
    /// Enforces the `emery` version floor: a pinned version newer than
    /// `CARGO_PKG_VERSION` is rejected, but an unparseable pin is
    /// tolerated — we prefer a permissive stance for a human-edited file.
    ///
    /// # Errors
    ///
    /// - [`Error::NotInitialized`] if `.emery/project.yaml` is absent.
    /// - [`Error::Io`] if the file exists but cannot be read.
    /// - [`Error::YamlDe`] if the file is not valid project YAML.
    /// - [`Error::CliTooOld`] if the pinned `emery` version floor is
    ///   newer than this binary's version.
    pub fn load(project_dir: &Path) -> Result<Self, Error> {
        let current = env!("CARGO_PKG_VERSION");
        let path = Layout::new(project_dir).config_path();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::NotInitialized);
            }
            Err(err) => return Err(Error::Io(err)),
        };

        let cfg: Self = serde_saphyr::from_str(&text)?;

        if let Some(required) = &cfg.emery_version
            && version_is_older(current, required)
        {
            return Err(Error::CliTooOld {
                required: required.clone(),
                found: current.to_string(),
            });
        }

        Ok(cfg)
    }

    /// Walk `start_dir` and its ancestors looking for the first directory
    /// that contains `.emery/project.yaml`. Returns `None` when no
    /// ancestor is initialised. Filesystem probe errors are treated as
    /// "this candidate isn't initialised" — the next ancestor is tried.
    #[must_use]
    pub fn find_root(start_dir: &Path) -> Option<PathBuf> {
        start_dir
            .ancestors()
            .find(|candidate| Layout::new(candidate).config_path().try_exists().unwrap_or(false))
            .map(Path::to_path_buf)
    }
}

/// Typed view over a project root that exposes every `.emery/` path
/// helper as an inherent method.
///
/// Construct with [`Layout::new`] (in-place) or [`Layout::with_change_root`].
/// Callers never join `.emery/...` literally. Durable product state
/// (`project.yaml`, `specs/`, `decisions/`) stays under `.emery/`;
/// change-scoped artifacts (`plan.yaml`, `change.md`, `discovery.yaml`,
/// `slices/`, `events/`, `targets/`, `archive/`, `guest.lock`) live
/// under [`Self::change_root`].
#[derive(Debug, Clone, Copy)]
pub struct Layout<'a> {
    project_dir: &'a Path,
    change_root: Option<&'a Path>,
}

impl<'a> Layout<'a> {
    /// Wrap `project_dir` as the typed in-place root for path lookups.
    #[must_use]
    pub const fn new(project_dir: &'a Path) -> Self {
        Self {
            project_dir,
            change_root: None,
        }
    }

    /// Layout whose change home is `change_root` (detached, or an
    /// explicit in-place override). `project_dir` is the `.` mount.
    #[must_use]
    pub const fn with_change_root(project_dir: &'a Path, change_root: &'a Path) -> Self {
        Self {
            project_dir,
            change_root: Some(change_root),
        }
    }

    /// Project root the layout is anchored at.
    #[must_use]
    pub const fn project_dir(&self) -> &'a Path {
        self.project_dir
    }

    /// Absolute path to `<project_dir>/.emery/`.
    #[must_use]
    pub fn emery_dir(&self) -> PathBuf {
        self.project_dir.join(".emery")
    }

    /// Absolute path to the change home: `<project_dir>/.emery/change/`
    /// in-place, or the operator directory when detached.
    #[must_use]
    pub fn change_root(&self) -> PathBuf {
        self.change_root.map_or_else(|| self.emery_dir().join(CHANGE_DIR_NAME), Path::to_path_buf)
    }

    /// Walk `<project>/.emery/change/slices/<name>/` up to the project
    /// root. `None` when the path is not a change-home slice directory.
    #[must_use]
    pub fn project_dir_from_slice(slice_dir: &Path) -> Option<PathBuf> {
        let slices = slice_dir.parent()?;
        if slices.file_name()? != std::ffi::OsStr::new(crate::slice::SLICES_DIR_NAME) {
            return None;
        }
        let change_root = slices.parent()?;
        if change_root.file_name()? != std::ffi::OsStr::new(CHANGE_DIR_NAME) {
            return None;
        }
        let emery_dir = change_root.parent()?;
        if emery_dir.file_name()? != std::ffi::OsStr::new(".emery") {
            return None;
        }
        emery_dir.parent().map(Path::to_path_buf)
    }

    /// Absolute path to `<project_dir>/.emery/project.yaml`.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.emery_dir().join("project.yaml")
    }

    /// Absolute path to `<project_dir>/.emery/specs/` — the baseline
    /// specs tree, one domain directory per `spec.md`.
    #[must_use]
    pub fn specs_dir(&self) -> PathBuf {
        self.emery_dir().join("specs")
    }

    /// Absolute path to `<project_dir>/.emery/change/slices/`.
    #[must_use]
    pub fn slices_dir(&self) -> PathBuf {
        self.change_root().join(crate::slice::SLICES_DIR_NAME)
    }

    /// Absolute path to one slice's working directory,
    /// `<project_dir>/.emery/change/slices/<name>/`.
    #[must_use]
    pub fn slice_dir(&self, name: &str) -> PathBuf {
        self.slices_dir().join(name)
    }

    /// Absolute path to `<project_dir>/.emery/decisions/` — the
    /// append-only Decision Record catalogue promoted by the execute
    /// loop's merge phase. One flat, project-global tree of
    /// `DEC-NNNN-<slug>.md` files. Machine-written by merge; the single
    /// permitted post-write mutation is a supersede status flip.
    #[must_use]
    pub fn decisions_dir(&self) -> PathBuf {
        self.emery_dir().join("decisions")
    }

    /// Absolute path to `<project_dir>/.emery/change/archive/`.
    #[must_use]
    pub fn archive_dir(&self) -> PathBuf {
        self.change_root().join("archive")
    }

    /// Absolute path to `<project_dir>/.emery/change/events/` —
    /// per-writer append-only fact logs (`<writer>.jsonl`).
    #[must_use]
    pub fn events_dir(&self) -> PathBuf {
        self.change_root().join("events")
    }

    /// Absolute path to one writer's event log,
    /// `<project_dir>/.emery/change/events/<writer>.jsonl`.
    #[must_use]
    pub fn writer_events_path(&self, writer: &str) -> PathBuf {
        self.events_dir().join(format!("{writer}.jsonl"))
    }

    /// Absolute path to `<project_dir>/.emery/change/targets/` —
    /// per-target wave manifests (RFC-86 D9).
    #[must_use]
    pub fn targets_dir(&self) -> PathBuf {
        self.change_root().join("targets")
    }

    /// Absolute path to
    /// `<project_dir>/.emery/change/targets/<target>/waves/`.
    #[must_use]
    pub fn target_waves_dir(&self, target: &str) -> PathBuf {
        self.targets_dir().join(target).join("waves")
    }

    /// Absolute path to one wave manifest,
    /// `<project_dir>/.emery/change/targets/<target>/waves/<digest>.yaml`
    /// where `digest` is the bare 64-hex content address (no `sha256:`
    /// scheme).
    #[must_use]
    pub fn target_wave_path(&self, target: &str, digest: &str) -> PathBuf {
        self.target_waves_dir(target).join(format!("{digest}.yaml"))
    }

    /// Absolute path to one slice's build-record directory,
    /// `<project_dir>/.emery/change/slices/<name>/builds/` (RFC-86 D27).
    #[must_use]
    pub fn slice_builds_dir(&self, name: &str) -> PathBuf {
        self.slice_dir(name).join("builds")
    }

    /// Absolute path to one content-addressed build record,
    /// `<project_dir>/.emery/change/slices/<name>/builds/<digest>.yaml`
    /// where `digest` is the bare 64-hex content address.
    #[must_use]
    pub fn slice_build_record_path(&self, name: &str, digest: &str) -> PathBuf {
        self.slice_builds_dir(name).join(format!("{digest}.yaml"))
    }

    /// Absolute path to `<project_dir>/.emery/change/plan.yaml`.
    #[must_use]
    pub fn plan_path(&self) -> PathBuf {
        self.change_root().join("plan.yaml")
    }

    /// Absolute path to the guest execute loop's create-exclusive
    /// advisory marker: `<change-root>/guest.lock`.
    #[must_use]
    pub fn guest_lock_path(&self) -> PathBuf {
        self.change_root().join("guest.lock")
    }

    /// Absolute path to `<project_dir>/.emery/change/change.md` — the
    /// umbrella operator brief beside `plan.yaml`.
    #[must_use]
    pub fn change_brief_path(&self) -> PathBuf {
        self.change_root().join("change.md")
    }

    /// Absolute path to `<change>/leads.md` — the authoritative lead catalog.
    #[must_use]
    pub fn leads_path(&self) -> PathBuf {
        self.change_root().join("leads.md")
    }

    /// Absolute path to `<change>/leads/` — retained catalog revisions.
    #[must_use]
    pub fn leads_dir(&self) -> PathBuf {
        self.change_root().join("leads")
    }

    /// Absolute path to `<change>/leads/<digest>.md`.
    #[must_use]
    pub fn leads_revision_path(&self, digest: &crate::snapshot::SnapshotId) -> PathBuf {
        self.leads_dir().join(format!("{}.md", digest.digest()))
    }

    /// Absolute path to `<change>/discovery.yaml` — pinned delivery topology.
    #[must_use]
    pub fn discovery_yaml_path(&self) -> PathBuf {
        self.change_root().join("discovery.yaml")
    }

    /// Absolute path to `<change>/decomposition.yaml`.
    #[must_use]
    pub fn decomposition_path(&self) -> PathBuf {
        self.change_root().join("decomposition.yaml")
    }

    /// Absolute path to `<change>/decompositions/` — retained revisions.
    #[must_use]
    pub fn decompositions_dir(&self) -> PathBuf {
        self.change_root().join("decompositions")
    }

    /// Absolute path to `<change>/decompositions/<digest>.yaml`.
    #[must_use]
    pub fn decomp_revision_path(&self, digest: &crate::snapshot::SnapshotId) -> PathBuf {
        self.decompositions_dir().join(format!("{}.yaml", digest.digest()))
    }

    /// Absolute path to `<change>/planning/proposals/`.
    #[must_use]
    pub fn proposals_dir(&self) -> PathBuf {
        self.change_root().join("planning").join("proposals")
    }

    /// Absolute path to `<change>/planning/proposals/<digest>.yaml`.
    #[must_use]
    pub fn proposal_path(&self, digest: &crate::snapshot::SnapshotId) -> PathBuf {
        self.proposals_dir().join(format!("{}.yaml", digest.digest()))
    }

    /// Absolute path to `<change>/imports/`.
    #[must_use]
    pub fn imports_dir(&self) -> PathBuf {
        self.change_root().join("imports")
    }

    /// Absolute path to `<change>/imports/handoffs/<digest>.yaml`.
    #[must_use]
    pub fn import_handoff_path(&self, digest: &crate::snapshot::SnapshotId) -> PathBuf {
        self.imports_dir().join("handoffs").join(format!("{}.yaml", digest.digest()))
    }

    /// Absolute path to `<change>/imports/reviews/<digest>.json`.
    #[must_use]
    pub fn import_review_path(&self, digest: &crate::snapshot::SnapshotId) -> PathBuf {
        self.imports_dir().join("reviews").join(format!("{}.json", digest.digest()))
    }
}

/// Returns `true` when `current < required` under semver ordering.
/// Unparseable versions are treated as "not older" — we don't want a
/// typo in a human-edited `project.yaml` to brick the project.
fn version_is_older(current: &str, required: &str) -> bool {
    let (Ok(cur), Ok(req)) = (semver::Version::parse(current), semver::Version::parse(required))
    else {
        return false;
    };
    cur < req
}
