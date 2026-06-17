//! `ProjectConfig` — in-memory model of `.specify/project.yaml` — and
//! `Layout<'a>`, the typed home for every `.specify/` and repo-root
//! path helper the CLI reaches for.

mod atomic;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use atomic::{AtomicYaml, with_state};
use serde::{Deserialize, Serialize};
use specify_error::Error;

use crate::platform::Platform;

/// In-memory representation of `.specify/project.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProjectConfig {
    /// Project name (defaults to the project directory name at init time).
    pub name: String,

    /// Free-text description of the project's tech stack, architecture,
    /// and testing approach. Falls back to the adapter's domain when empty.
    ///
    /// Authored intent only. A project's *derived* routing identity —
    /// the `surface[]` of owned domains and a `recent[]` merge tail — is
    /// projected from its baseline (`.specify/specs/` + journal), never
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

    /// Minimum `specify` CLI version required to operate on this project.
    /// Written by `specify init` as the running binary's version and
    /// enforced by [`ProjectConfig::load`] via the `semver` crate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specify_version: Option<String>,

    /// Map of brief id (e.g. `proposal`, `specs`, `design`, `tasks`) to a
    /// path (relative to `.specify/`) of a markdown file containing extra
    /// rules for that brief. Scaffolded with one empty entry per
    /// `pipeline.define` brief by `specify init`.
    #[serde(default)]
    pub rules: BTreeMap<String, String>,

    /// Project-scope WASI tool declarations. These are generic extension
    /// points owned by `specify-tool`, not by any adapter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<specify_extension_manifest::Extension>,

    /// Target platforms this project builds for (e.g. `core`, `ios`,
    /// `android`). Set at `specify init --platforms` and changeable via
    /// `specify init --upgrade --platforms`. When the bound target
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
    /// Load `.specify/project.yaml` from `project_dir`.
    ///
    /// Enforces the `specify_version` floor: a pinned version newer than
    /// `CARGO_PKG_VERSION` is rejected, but an unparseable pin is
    /// tolerated — we prefer a permissive stance for a human-edited file.
    ///
    /// # Errors
    ///
    /// - [`Error::NotInitialized`] if `.specify/project.yaml` is absent.
    /// - [`Error::Io`] if the file exists but cannot be read.
    /// - [`Error::YamlDe`] if the file is not valid project YAML.
    /// - [`Error::CliTooOld`] if the pinned `specify_version` floor is
    ///   newer than this binary's version.
    pub fn load(project_dir: &Path) -> Result<Self, Error> {
        Self::load_with_current(project_dir, env!("CARGO_PKG_VERSION"))
    }

    /// Version-injectable body of [`ProjectConfig::load`]; `current` is
    /// the running binary's version. Split out so the `CliTooOld` floor
    /// check keeps unit coverage against arbitrary versions.
    fn load_with_current(project_dir: &Path, current: &str) -> Result<Self, Error> {
        let path = Layout::new(project_dir).config_path();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::NotInitialized);
            }
            Err(err) => return Err(Error::Io(err)),
        };

        let cfg: Self = serde_saphyr::from_str(&text)?;

        if let Some(required) = &cfg.specify_version
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
    /// that contains `.specify/project.yaml`. Returns `None` when no
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

/// Typed view over a project root that exposes every `.specify/` and
/// repo-root path helper as an inherent method.
///
/// Construct with [`Layout::new`]. The newtype concentrates the
/// `.specify/` boundary in one place: callers never join
/// `.specify/...` literally; they ask the layout for the directory
/// they want.
///
/// The **plan root** (where `plan.yaml`, `change.md`, and
/// `discovery.md` live) defaults to `project_dir` and is overridable
/// with [`Layout::with_plan_dir`]: during workspace-routed phase work
/// the plan artifacts live at the initiating workspace, not the slot,
/// and slot-side verbs receive the workspace root via the global
/// `--plan-dir` flag (env `SPECIFY_PLAN_DIR`).
#[derive(Debug, Clone, Copy)]
pub struct Layout<'a> {
    project_dir: &'a Path,
    plan_dir: Option<&'a Path>,
}

impl<'a> Layout<'a> {
    /// Wrap `project_dir` as the typed root for path lookups.
    #[must_use]
    pub const fn new(project_dir: &'a Path) -> Self {
        Self {
            project_dir,
            plan_dir: None,
        }
    }

    /// Override the plan root: `plan.yaml`, `change.md`, and
    /// `discovery.md` resolve against `plan_dir` instead of the
    /// project root. `None` leaves the default in place.
    #[must_use]
    pub const fn with_plan_dir(mut self, plan_dir: Option<&'a Path>) -> Self {
        self.plan_dir = plan_dir;
        self
    }

    /// The plan root: `plan_dir` when overridden, the project root
    /// otherwise.
    #[must_use]
    pub const fn plan_dir(&self) -> &'a Path {
        match self.plan_dir {
            Some(dir) => dir,
            None => self.project_dir,
        }
    }

    /// Project root the layout is anchored at.
    #[must_use]
    pub const fn project_dir(&self) -> &'a Path {
        self.project_dir
    }

    /// Absolute path to `<project_dir>/.specify/`.
    #[must_use]
    pub fn specify_dir(&self) -> PathBuf {
        self.project_dir.join(".specify")
    }

    /// Absolute path to `<project_dir>/.specify/project.yaml`.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.specify_dir().join("project.yaml")
    }

    /// Absolute path to `<project_dir>/.specify/slices/`.
    #[must_use]
    pub fn slices_dir(&self) -> PathBuf {
        self.specify_dir().join(crate::slice::SLICES_DIR_NAME)
    }

    /// Absolute path to `<project_dir>/.specify/topology.lock` — the
    /// committed projection of each member project's `project.yaml`
    /// topology facets, regenerated by `specify workspace sync`.
    /// Machine-written; never hand-edited.
    #[must_use]
    pub fn topology_lock_path(&self) -> PathBuf {
        self.specify_dir().join("topology.lock")
    }

    /// Absolute path to this project's out-of-tree memoization root
    /// (manifest mirror, codex, …), resolved from the OS cache via
    /// [`specify_schema::cache::project_cache_dir`]. Lives outside the
    /// working tree, keyed by a digest of the project path, so deleting
    /// it costs recomputation only and it never pollutes git. Transient
    /// per-run working state lives in-tree under [`Self::scratch_dir`].
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        specify_schema::cache::project_cache_dir(self.project_dir)
    }

    /// Absolute path to `<project_dir>/.specify/scratch/` — the
    /// transient working-state root. Per-run lanes only (source
    /// operation `$SCRATCH_DIR` lanes, the plan handoff lane); every
    /// lane is recreated empty by its owning verb, so the tree can be
    /// wiped at any time at zero cost. Disjoint from
    /// [`Self::cache_dir`] so a scratch write can never pollute a
    /// cache artifact. See DECISIONS.md §"Cache layout".
    #[must_use]
    pub fn scratch_dir(&self) -> PathBuf {
        self.specify_dir().join("scratch")
    }

    /// Absolute path to `<project_dir>/.specify/scratch/plan/` — the
    /// plan-phase handoff lane. `specify plan propose --dry-run`
    /// recreates it empty; the agent writes the reconciliation
    /// response envelope (`propose-response.json`) into it for
    /// `specify plan propose --from`.
    #[must_use]
    pub fn plan_scratch_dir(&self) -> PathBuf {
        self.scratch_dir().join("plan")
    }

    /// Absolute path to `<project_dir>/.specify/decisions/` — the
    /// append-only Decision Record catalogue promoted by
    /// `specify slice merge`. One flat, project-global tree of
    /// `DEC-NNNN-<slug>.md` files. Machine-written by merge; the single
    /// permitted post-write mutation is a supersede status flip.
    #[must_use]
    pub fn decisions_dir(&self) -> PathBuf {
        self.specify_dir().join("decisions")
    }

    /// Absolute path to `<project_dir>/.specify/archive/`. Centralised
    /// here so there is exactly one place the convention lives.
    #[must_use]
    pub fn archive_dir(&self) -> PathBuf {
        self.specify_dir().join("archive")
    }

    /// Absolute path to `<project_dir>/registry.yaml` — the platform
    /// catalogue. Platform-level artifact, lives at the repo root.
    #[must_use]
    pub fn registry_path(&self) -> PathBuf {
        self.project_dir.join("registry.yaml")
    }

    /// Absolute path to `<plan-root>/plan.yaml` — the change plan.
    /// Platform-level artifact at the repo root, or at the initiating
    /// workspace root when the plan root is overridden
    /// ([`Layout::with_plan_dir`]).
    #[must_use]
    pub fn plan_path(&self) -> PathBuf {
        self.plan_dir().join("plan.yaml")
    }

    /// Absolute path to `<plan-root>/.specify/plan.lock` — the
    /// skill-acquired `/spec:execute` driver lock
    /// ([`crate::plan_lock`]). Anchored at the plan root so slot-side
    /// phase work under `--plan-dir` probes the *workspace* lock.
    #[must_use]
    pub fn plan_lock_path(&self) -> PathBuf {
        self.plan_dir().join(".specify").join("plan.lock")
    }

    /// Absolute path to `<plan-root>/change.md` — the umbrella
    /// operator brief beside `plan.yaml`. Platform-level artifact.
    #[must_use]
    pub fn change_brief_path(&self) -> PathBuf {
        self.plan_dir().join("change.md")
    }

    /// Absolute path to `<plan-root>/discovery.md` — the candidate
    /// inventory written at `/spec:plan`'s survey step and read during
    /// lead reconciliation. Lives beside `plan.yaml`.
    #[must_use]
    pub fn discovery_path(&self) -> PathBuf {
        self.plan_dir().join("discovery.md")
    }
}

/// Detect whether `project_dir` is, or lives below, a materialised
/// workspace slot at `<platform>/workspace/<peer>/`.
///
/// A slot is identified structurally: some ancestor's immediate parent
/// is a `workspace/` directory whose own parent (the platform root)
/// carries a `.specify/project.yaml`. The platform-config check
/// disambiguates a real slot from an ordinary project that merely sits
/// inside a directory named `workspace`, so this necessarily touches
/// the filesystem. Context generation uses the shared posture to skip
/// init-time `AGENTS.md` creation in materialised slots; callers that
/// need a fully initialized slot can layer plan-file guards on top.
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
            platform_root.join(".specify").join("project.yaml").is_file()
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

#[cfg(test)]
mod tests;
