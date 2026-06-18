//! Parser and types for `registry.yaml` — the platform-level catalogue
//! of peer projects and their adapters. Shape is enforced by
//! [`Registry::validate_shape`] (in [`crate::registry::validate`]).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specify_error::Error;

/// In-memory representation of `registry.yaml` (at the repo root).
///
/// `additionalProperties: false` is expressed via
/// `#[serde(deny_unknown_fields)]` — the same posture the `plan.yaml`
/// `ScopeShape` uses — so typos (e.g. `versions:`, `project:`) fail
/// fast at parse time rather than silently round-tripping.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// Schema version. `1` is the only accepted value for this
    /// release; [`Registry::validate_shape`] rejects anything else
    /// with an actionable diagnostic.
    pub version: u32,
    /// Platform catalogue. Empty or single-entry is equivalent to
    /// "single-repo mode"; multi-entry activates the `/change:draft`
    /// *sync workspace* phase (C28/C30).
    #[serde(default)]
    pub projects: Vec<RegistryProject>,
}

/// One entry in [`Registry::projects`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryProject {
    /// Kebab-case identifier for the project; validated by
    /// [`specify_error::is_kebab`].
    pub name: String,
    /// Clone target — `.`, a repo-relative path (`../peer`, `./foo`,
    /// `pkg/sub`), `git@host:path`, or an `http(s)://`, `ssh://`, or
    /// `git+http(s)://` / `git+ssh://` remote. Shape-validated by
    /// [`Registry::validate_shape`]. Stored verbatim.
    pub url: String,
    /// Optional greenfield scaffold seed — the adapter written
    /// into a brand-new project's `project.yaml` when `workspace sync`
    /// clones an empty repo. **Not** read for plan-time topology; a
    /// project's authoritative target adapter lives in its own
    /// `project.yaml` and is projected into `.specify/topology.lock`.
    /// Opaque at this layer; the `name@version` suffix is not parsed
    /// here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// A greenfield seed; ignored at topology time.
    /// A project's authoritative description lives in its own
    /// `project.yaml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional contract role declarations for this project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contracts: Option<ContractRoles>,
    /// RFC-46 D6 — optional greenfield identity seed. Carries the
    /// project's intended domain slugs so a fresh project with no
    /// baseline (`.specify/specs/` absent) still routes leads at plan
    /// time — the greenfield analog of the projected `surface[]` domain
    /// list. The seed projects into an empty `ProjectRef.surface[]` and
    /// is ignored once a real baseline exists. Carries domain slugs
    /// only; adapter/description material lives in `project.yaml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub greenfield_seed: Option<GreenfieldSeed>,
}

/// RFC-46 D6 greenfield identity seed — see
/// [`RegistryProject::greenfield_seed`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GreenfieldSeed {
    /// Intended domain slugs (kebab-case), validated by
    /// [`Registry::validate_shape`]. Each projects into a `surface[]`
    /// entry with empty `requirements[]` until the real baseline
    /// supersedes it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub domains: Vec<String>,
}

/// Contract role declarations for a registry project.
/// All fields are optional — a project may only produce, only consume,
/// or have no contract relationships at all.
///
/// The role set is exactly two: `produces` (this project authoritatively
/// implements the contract) and `consumes` (this project calls or
/// subscribes to the contract). A contract that no project produces is,
/// by definition, externally authored — no separate `imports` field is
/// needed to mark it. `#[serde(deny_unknown_fields)]` causes any
/// surviving `imports:` key in `registry.yaml` to fail at parse time.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContractRoles {
    /// Contract files this project is the authoritative implementer of.
    /// Paths relative to root `contracts/`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub produces: Vec<String>,
    /// Contract files this project calls or subscribes to as a client.
    /// Paths relative to root `contracts/`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes: Vec<String>,
}

impl Registry {
    /// Absolute path to `<project_dir>/registry.yaml`. The platform
    /// catalogue lives at the repo root.
    #[must_use]
    pub fn path(project_dir: &Path) -> PathBuf {
        project_dir.join("registry.yaml")
    }

    /// Load + shape-validate the registry. A missing file is *not* an
    /// error — the registry is optional and yields `Ok(None)`.
    ///
    /// # Errors
    ///
    /// - [`Error::Diag`] `registry-read-failed` if the file exists but
    ///   cannot be read.
    /// - [`Error::Diag`] `registry-malformed` if the YAML is invalid or
    ///   carries unknown keys.
    /// - The first shape violation from [`Registry::validate_shape`]
    ///   (wrong `version`, kebab-case / required-field / duplicate-name).
    pub fn load(project_dir: &Path) -> Result<Option<Self>, Error> {
        let path = Self::path(project_dir);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path).map_err(|err| Error::Diag {
            code: "registry-read-failed",
            detail: format!("failed to read {}: {err}", path.display()),
        })?;
        let registry: Self = serde_saphyr::from_str(&content).map_err(|err| Error::Diag {
            code: "registry-malformed",
            detail: format!("registry.yaml: invalid YAML: {err}"),
        })?;
        registry.validate_shape()?;
        Ok(Some(registry))
    }

    /// `true` when the registry declares at most one project.
    ///
    /// Absent registry + single-entry registry behave identically in
    /// the `/change:draft` flow. Useful where the *sync workspace* phase is
    /// gated on `len(projects) > 1`.
    #[must_use]
    pub const fn is_single_repo(&self) -> bool {
        self.projects.len() <= 1
    }

    /// Resolve optional project selectors against `registry.yaml`.
    ///
    /// Empty selectors mean every registry project. Non-empty selectors
    /// are treated as a set, but output always follows registry order so
    /// workspace verbs behave consistently regardless of CLI argument
    /// order.
    ///
    /// # Errors
    ///
    /// Returns an error if the registry shape is invalid or any selector
    /// does not match a declared project name.
    pub fn select<'a>(&'a self, selectors: &[String]) -> Result<Vec<&'a RegistryProject>, Error> {
        self.validate_shape()?;
        if selectors.is_empty() {
            return Ok(self.projects.iter().collect());
        }

        let requested: HashSet<&str> = selectors.iter().map(String::as_str).collect();
        let selected: Vec<&RegistryProject> = self
            .projects
            .iter()
            .filter(|project| requested.contains(project.name.as_str()))
            .collect();

        if selected.len() == requested.len() {
            return Ok(selected);
        }

        let matched: HashSet<&str> = selected.iter().map(|project| project.name.as_str()).collect();
        let mut unknown = Vec::new();
        for selector in selectors {
            let name = selector.as_str();
            if !matched.contains(name) && !unknown.contains(&name) {
                unknown.push(name);
            }
        }

        let known = self
            .projects
            .iter()
            .map(|project| project.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let unknown_list =
            unknown.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ");
        let noun = if unknown.len() == 1 { "selector" } else { "selectors" };
        Err(Error::Diag {
            code: "registry-project-selector-unknown",
            detail: format!(
                "registry.yaml: unknown project {noun} {unknown_list}; expected one of: {known}"
            ),
        })
    }
}

impl RegistryProject {
    /// `true` when this entry's [`RegistryProject::url`] should be
    /// materialised under `workspace/<name>/` as a symlink to a
    /// resolved filesystem path (`.` or a repo-relative path), as opposed
    /// to a `git clone` remote.
    ///
    /// Callers may assume [`Registry::validate_shape`] has already accepted
    /// the URL — this predicate mirrors the C28 classification rules.
    #[must_use]
    pub fn is_local(&self) -> bool {
        self.url == "." || (!self.url.contains("://") && !self.url.starts_with("git@"))
    }
}
