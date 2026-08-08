//! `ProjectConfig` — in-memory model of `.emery/project.yaml` — and
//! `Layout<'a>`, the typed home for every `.emery/` and repo-root
//! path helper the CLI reaches for.

mod atomic;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use atomic::{AtomicYaml, Mutation, with_state};
use error::Error;
use serde::{Deserialize, Serialize};

use crate::platform::Platform;

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
    /// Absent for registry-only workspaces (`workspace: true`); see the
    /// `workspace` field for the discriminator.
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

    /// `true` when this project is a registry-only **workspace**.
    /// Workspaces hold platform-level state — `registry.yaml`,
    /// `change.md`, `plan.yaml`, workspace slots under `workspace/`
    /// — but never appear in their own `registry.yaml` and have phase
    /// pipelines disabled. Workspaces **omit** the `adapter:` field
    /// entirely; the absence of `adapter:` together with `workspace: true`
    /// is the discriminator. Defaults to `false`; serialised only when
    /// `true` so regular `project.yaml` files round-trip byte-stable.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub workspace: bool,
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

/// Typed view over a project root that exposes every `.emery/` and
/// repo-root path helper as an inherent method.
///
/// Construct with [`Layout::new`]. The newtype concentrates the
/// `.emery/` boundary in one place: callers never join
/// `.emery/...` literally; they ask the layout for the directory
/// they want. Plan artifacts (`plan.yaml`, `change.md`,
/// `discovery.md`) anchor at the invoked project directory alongside
/// everything else.
#[derive(Debug, Clone, Copy)]
pub struct Layout<'a> {
    project_dir: &'a Path,
}

impl<'a> Layout<'a> {
    /// Wrap `project_dir` as the typed root for path lookups.
    #[must_use]
    pub const fn new(project_dir: &'a Path) -> Self {
        Self { project_dir }
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

    /// Absolute path to `<project_dir>/.emery/slices/`.
    #[must_use]
    pub fn slices_dir(&self) -> PathBuf {
        self.emery_dir().join(crate::slice::SLICES_DIR_NAME)
    }

    /// Absolute path to one slice's working directory,
    /// `<project_dir>/.emery/slices/<name>/`.
    #[must_use]
    pub fn slice_dir(&self, name: &str) -> PathBuf {
        self.slices_dir().join(name)
    }

    /// Absolute path to `<project_dir>/.emery/topology.lock` — the
    /// committed projection of each member project's `project.yaml`
    /// topology facets projected from the workspace registry.
    /// Machine-written; never hand-edited.
    #[must_use]
    pub fn topology_lock_path(&self) -> PathBuf {
        self.emery_dir().join("topology.lock")
    }

    /// Absolute path to `<project_dir>/.emery/decisions/` — the
    /// append-only Decision Record catalogue promoted by
    /// `emery slice merge`. One flat, project-global tree of
    /// `DEC-NNNN-<slug>.md` files. Machine-written by merge; the single
    /// permitted post-write mutation is a supersede status flip.
    #[must_use]
    pub fn decisions_dir(&self) -> PathBuf {
        self.emery_dir().join("decisions")
    }

    /// Absolute path to `<project_dir>/.emery/archive/`. Centralised
    /// here so there is exactly one place the convention lives.
    #[must_use]
    pub fn archive_dir(&self) -> PathBuf {
        self.emery_dir().join("archive")
    }

    /// Absolute path to `<project_dir>/.emery/events/` — per-writer
    /// append-only fact logs (`<writer>.jsonl`). Pre-RFC-88 stand-in
    /// home (flat `.emery/`); the two-root cut moves these under the
    /// change tree.
    #[must_use]
    pub fn events_dir(&self) -> PathBuf {
        self.emery_dir().join("events")
    }

    /// Absolute path to one writer's event log,
    /// `<project_dir>/.emery/events/<writer>.jsonl`.
    #[must_use]
    pub fn writer_events_path(&self, writer: &str) -> PathBuf {
        self.events_dir().join(format!("{writer}.jsonl"))
    }

    /// Absolute path to `<project_dir>/.emery/targets/` — per-target
    /// wave manifests (RFC-86 D9). Pre-RFC-88 stand-in home under the
    /// flat `.emery/` root.
    #[must_use]
    pub fn targets_dir(&self) -> PathBuf {
        self.emery_dir().join("targets")
    }

    /// Absolute path to `<project_dir>/.emery/targets/<target>/waves/`.
    #[must_use]
    pub fn target_waves_dir(&self, target: &str) -> PathBuf {
        self.targets_dir().join(target).join("waves")
    }

    /// Absolute path to one wave manifest,
    /// `<project_dir>/.emery/targets/<target>/waves/<digest>.yaml`
    /// where `digest` is the bare 64-hex content address (no `sha256:`
    /// scheme).
    #[must_use]
    pub fn target_wave_path(&self, target: &str, digest: &str) -> PathBuf {
        self.target_waves_dir(target).join(format!("{digest}.yaml"))
    }

    /// Absolute path to one slice's build-record directory,
    /// `<project_dir>/.emery/slices/<name>/builds/` (RFC-86 D27).
    #[must_use]
    pub fn slice_builds_dir(&self, name: &str) -> PathBuf {
        self.slice_dir(name).join("builds")
    }

    /// Absolute path to one content-addressed build record,
    /// `<project_dir>/.emery/slices/<name>/builds/<digest>.yaml`
    /// where `digest` is the bare 64-hex content address.
    #[must_use]
    pub fn slice_build_record_path(&self, name: &str, digest: &str) -> PathBuf {
        self.slice_builds_dir(name).join(format!("{digest}.yaml"))
    }

    /// Absolute path to `<project_dir>/registry.yaml` — the platform
    /// catalogue. Platform-level artifact, lives at the repo root.
    #[must_use]
    pub fn registry_path(&self) -> PathBuf {
        self.project_dir.join("registry.yaml")
    }

    /// Absolute path to `<project_dir>/plan.yaml` — the change plan.
    /// Platform-level artifact at the repo root.
    #[must_use]
    pub fn plan_path(&self) -> PathBuf {
        self.project_dir.join("plan.yaml")
    }

    /// Absolute path to `<project_dir>/.emery/guest.lock` — the
    /// guest execute loop's create-exclusive advisory marker
    /// (the engine guest marker), the guest-vs-guest
    /// breakout refusal fence.
    #[must_use]
    pub fn guest_lock_path(&self) -> PathBuf {
        self.emery_dir().join("guest.lock")
    }

    /// Absolute path to `<project_dir>/change.md` — the umbrella
    /// operator brief beside `plan.yaml`. Platform-level artifact.
    #[must_use]
    pub fn change_brief_path(&self) -> PathBuf {
        self.project_dir.join("change.md")
    }

    /// Absolute path to `<project_dir>/discovery.md` — the candidate
    /// inventory written at `/emery:plan`'s survey step and read during
    /// lead reconciliation. Lives beside `plan.yaml`.
    #[must_use]
    pub fn discovery_path(&self) -> PathBuf {
        self.project_dir.join("discovery.md")
    }
}

/// Detect whether `project_dir` is, or lives below, a materialised
/// workspace slot at `<platform>/workspace/<peer>/`.
///
/// A slot is identified structurally: some ancestor's immediate parent
/// is a `workspace/` directory whose own parent carries a
/// `.emery/project.yaml`. That platform-config check (a filesystem
/// touch) disambiguates a real slot from an ordinary project that
/// merely sits inside a directory named `workspace`.
#[must_use]
pub fn is_slot(project_dir: &Path) -> bool {
    project_dir.ancestors().any(|candidate| {
        let Some(workspace) = candidate.parent() else {
            return false;
        };
        if workspace.file_name() != Some(std::ffi::OsStr::new("workspace")) {
            return false;
        }
        workspace.parent().is_some_and(|platform_root| {
            platform_root.join(".emery").join("project.yaml").is_file()
        })
    })
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
