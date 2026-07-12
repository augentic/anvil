//! `specify registry *` — validated edits to `registry.yaml` inside
//! the project root. Pure filesystem, so the verbs run on every
//! transport.

use std::io::Write;
use std::path::{Path, PathBuf};

use artifacts::atomic::yaml_write;
use error::{Error, Result};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use super::{Registry, RegistryProject};
use crate::change::Plan;
use crate::config::{Layout, Mutation, ProjectConfig, with_state};
use crate::handler::{Anchor, Ctx, Render};

/// Wire input for `registry validate` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct ValidateInput {}

/// `specify registry validate` — validate `registry.yaml` shape.
/// Absent file exits 0.
#[derive(Clone, Copy, Debug)]
pub struct Validate;

impl<P: Anchor> Operation<P> for Validate {
    type Error = crate::handler::Error;
    type Input = ValidateInput;
    type Output = ValidateBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        // `specify registry validate` is allowed to run before `specify
        // init`, so it anchors directly from the provider instead of
        // loading `Ctx` (which requires `.specify/project.yaml`). An
        // initialised ancestor still wins so the verb sees the same
        // root as every other command — and its config load failures
        // (parse errors, the `specify:` version floor) still propagate;
        // only the genuinely uninitialised case falls back to the
        // anchor directory with the base (non-workspace) shape check.
        let anchor = context.provider.project_root();
        let (project_dir, workspace_mode) = match ProjectConfig::find_root(anchor) {
            Some(root) => {
                let workspace = ProjectConfig::load(&root)?.workspace;
                (root, workspace)
            }
            None => (anchor.to_path_buf(), false),
        };
        let path = Registry::path(&project_dir);
        let registry = Registry::load(&project_dir)?;
        if workspace_mode && let Some(reg) = registry.as_ref() {
            reg.validate_workspace()?;
        }
        Ok(ValidateBody {
            registry,
            path,
            workspace_mode,
        })
    }
}

/// Success envelope for `registry validate`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ValidateBody {
    /// Parsed registry, when present.
    pub registry: Option<Registry>,
    /// Validated file location.
    pub path: PathBuf,
    /// Shape policy applied during validation.
    #[serde(skip)]
    pub workspace_mode: bool,
}

impl Render for ValidateBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let Some(reg) = self.registry.as_ref() else {
            return writeln!(w, "no registry declared at registry.yaml");
        };
        let count = reg.projects.len();
        if self.workspace_mode {
            writeln!(w, "registry.yaml is well-formed in workspace mode ({count} project(s))")
        } else {
            writeln!(w, "registry.yaml is well-formed ({count} project(s))")
        }
    }
}

/// Wire input for `registry add`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddInput {
    /// Kebab-case project name. Must be unique within the registry.
    pub name: String,
    /// Clone target — `.`, a repo-relative path, `git@host:path`, or a
    /// remote URL.
    pub url: String,
    /// Optional greenfield scaffold seed adapter.
    #[serde(default)]
    pub adapter: Option<String>,
    /// Optional greenfield seed description.
    #[serde(default)]
    pub description: Option<String>,
}

/// `specify registry add <name> --url <url>` — append a new project
/// entry to `registry.yaml`, creating the file when absent.
#[derive(Clone, Copy, Debug)]
pub struct Add;

impl<P: Anchor> Operation<P> for Add {
    type Error = crate::handler::Error;
    type Input = AddInput;
    type Output = AddBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let AddInput {
            name,
            url,
            adapter,
            description,
        } = input;
        if !crate::name::is_kebab(&name) {
            return Err(Error::Diag {
                code: "registry-add-name-not-kebab",
                detail: format!(
                    "registry add: project name `{name}` must be kebab-case \
                     (lowercase ascii, digits, single hyphens; no leading/trailing/doubled \
                     hyphens)"
                ),
            }
            .into());
        }

        let registry_path = Registry::path(&cx.project_dir);
        let workspace_mode = cx.config.workspace;
        // `--adapter` is an optional greenfield scaffold seed only.
        let candidate = RegistryProject {
            name,
            url,
            adapter: adapter.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            description: description.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            contracts: None,
            greenfield_seed: None,
        };

        // `registry add` is "create or update": an absent `registry.yaml`
        // is synthesised from the canonical empty shape so the first
        // `add` against a fresh project succeeds without a separate
        // bootstrap step.
        let mut registry = Registry::load(&cx.project_dir)?.unwrap_or_else(|| Registry {
            version: 1,
            projects: Vec::new(),
        });

        if registry.projects.iter().any(|p| p.name == candidate.name) {
            return Err(Error::Diag {
                code: "registry-add-name-duplicate",
                detail: format!(
                    "registry add: project `{}` already exists in {}",
                    candidate.name,
                    registry_path.display()
                ),
            }
            .into());
        }

        let added = candidate.clone();
        registry.projects.push(candidate);

        // Surface validate_shape / validate_workspace errors verbatim —
        // their diagnostic codes (`description-missing-multi-repo`,
        // `workspace-cannot-be-project`, etc.) are the documented contract.
        // Returning Err here aborts before the atomic write, so the
        // on-disk registry is never left in a shape-invalid state.
        if workspace_mode {
            registry.validate_workspace()?;
        } else {
            registry.validate_shape()?;
        }

        yaml_write(&registry_path, &registry)?;

        Ok(AddBody {
            registry,
            path: registry_path,
            added,
        })
    }
}

/// Success envelope for `registry add`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddBody {
    /// Persisted registry.
    pub registry: Registry,
    /// Updated file location.
    pub path: PathBuf,
    /// Appended entry.
    pub added: RegistryProject,
}

impl Render for AddBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "Added `{}` to {}", self.added.name, self.path.display())?;
        writeln!(w, "registry now declares {} project(s)", self.registry.projects.len())
    }
}

/// Wire input for `registry remove`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveInput {
    /// Project entry to remove.
    pub name: String,
}

/// `specify registry remove <name>` — remove an existing project
/// entry. Warns when `plan.yaml` references it.
#[derive(Clone, Copy, Debug)]
pub struct Remove;

impl<P: Anchor> Operation<P> for Remove {
    type Error = crate::handler::Error;
    type Input = RemoveInput;
    type Output = RemoveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let name = input.name;
        let path = Registry::path(&cx.project_dir);
        let workspace_mode = cx.config.workspace;

        // Pre-flight: surface the `registry-remove-no-registry`
        // diagnostic when the file is absent. `with_state` would
        // emit the generic `Error::ArtifactNotFound`; the registry-specific
        // diag is part of the wire contract.
        if !path.exists() {
            return Err(Error::Diag {
                code: "registry-remove-no-registry",
                detail: format!("registry remove: no registry declared at {}", path.display()),
            }
            .into());
        }

        let project_dir = cx.project_dir.clone();
        let body = with_state::<Registry, _, _>(cx.layout(), "registry.yaml", move |registry| {
            let position =
                registry.projects.iter().position(|p| p.name == name).ok_or_else(|| {
                    Error::Diag {
                        code: "registry-remove-not-found",
                        detail: format!(
                            "registry remove: project `{name}` not found in {}",
                            path.display()
                        ),
                    }
                })?;
            registry.projects.remove(position);

            // A removal can only relax the multi-repo description
            // invariant, so the post-write check should always
            // succeed; we run it anyway to pin the contract.
            if workspace_mode {
                registry.validate_workspace()?;
            } else {
                registry.validate_shape()?;
            }

            let warnings = plan_refs(&project_dir, &name);
            Ok(Mutation::changed(RemoveBody {
                registry: registry.clone(),
                path,
                removed: name,
                warnings,
            }))
        })?;

        Ok(body)
    }
}

/// Success envelope for `registry remove`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveBody {
    /// Persisted registry.
    pub registry: Registry,
    /// Updated file location.
    pub path: PathBuf,
    /// Removed project.
    pub removed: String,
    /// Advisory warnings (stale `plan.yaml` references).
    pub warnings: Vec<String>,
}

impl Render for RemoveBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "Removed `{}` from {}", self.removed, self.path.display())?;
        for warning in &self.warnings {
            writeln!(w, "warning: {warning}")?;
        }
        Ok(())
    }
}

/// Scan `plan.yaml` (when present) for plan entries whose `project`
/// field equals `removed`. Returns one human-readable warning per
/// affected entry. Best-effort: any parse error is surfaced as a
/// single advisory string instead of failing the remove (the registry
/// write has already landed, so the operator needs to learn about
/// both halves).
fn plan_refs(project_dir: &Path, removed: &str) -> Vec<String> {
    let plan_path = Layout::new(project_dir).plan_path();
    if !plan_path.exists() {
        return Vec::new();
    }
    match Plan::load(&plan_path) {
        Ok(plan) => {
            let referencing: Vec<&str> = plan
                .entries
                .iter()
                .filter(|entry| entry.project.as_deref() == Some(removed))
                .map(|entry| entry.name.as_str())
                .collect();
            if referencing.is_empty() {
                Vec::new()
            } else {
                vec![format!(
                    "plan.yaml has {n} entry(ies) still referencing project `{removed}`: {entries}. \
                     Run `specify plan amend <change> --project <other>` to rewire them.",
                    n = referencing.len(),
                    entries = referencing.join(", "),
                )]
            }
        }
        Err(err) => vec![format!(
            "plan.yaml present but unreadable; cannot check for stale references to `{removed}`: {err}"
        )],
    }
}
