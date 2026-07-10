//! `specify registry *` — validated edits to `registry.yaml` inside
//! the project root. Pure filesystem, so the verbs run on every
//! transport.

use std::io::Write;
use std::path::Path;

use artifacts::atomic::yaml_write;
use error::{Error, Result};
use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};
use super::{Registry, RegistryProject};
use crate::change::Plan;
use crate::config::{Layout, ProjectConfig, with_state};
use crate::verb::{Anchor, Ctx, Out, Render};

// ---------------------------------------------------------------------------
// registry validate
// ---------------------------------------------------------------------------

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

impl<P: Anchor> Handler<P> for Validate {
    type Error = crate::verb::Error;
    type Input = ValidateInput;
    type Output = Out<ValidateBody>;

    fn from_input(_: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self)
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let path = Registry::path(&cx.project_dir).display().to_string();
        // Workspaces opt into the stricter shape via `project.yaml:workspace:
        // true`. Tolerate a missing/unparseable project.yaml here —
        // `specify registry validate` is allowed to run before `specify
        // init`, in which case there is no workspace flag to honour and the base
        // shape check is the right behaviour.
        let workspace_mode = ProjectConfig::load(&cx.project_dir).is_ok_and(|cfg| cfg.workspace);
        let registry = Registry::load(&cx.project_dir)?;
        if workspace_mode && let Some(reg) = registry.as_ref() {
            reg.validate_shape_workspace()?;
        }
        Ok(Reply::ok(Out(ValidateBody {
            registry,
            path,
            workspace_mode,
        })))
    }
}

/// Success envelope for `registry validate`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ValidateBody {
    /// The loaded registry, when one exists.
    pub registry: Option<Registry>,
    /// Display path of `registry.yaml`.
    pub path: String,
    /// Whether the workspace shape rules applied.
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

// ---------------------------------------------------------------------------
// registry add
// ---------------------------------------------------------------------------

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
#[derive(Debug)]
pub struct Add {
    input: AddInput,
}

impl<P: Anchor> Handler<P> for Add {
    type Error = crate::verb::Error;
    type Input = AddInput;
    type Output = Out<AddBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let AddInput {
            name,
            url,
            adapter,
            description,
        } = self.input;
        if !error::is_kebab(&name) {
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
        let path = registry_path.display().to_string();
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
                    "registry add: project `{}` already exists in {path}",
                    candidate.name
                ),
            }
            .into());
        }

        let added = candidate.clone();
        registry.projects.push(candidate);

        // Surface validate_shape / validate_shape_workspace errors verbatim —
        // their diagnostic codes (`description-missing-multi-repo`,
        // `workspace-cannot-be-project`, etc.) are the documented contract.
        // Returning Err here aborts before the atomic write, so the
        // on-disk registry is never left in a shape-invalid state.
        if workspace_mode {
            registry.validate_shape_workspace()?;
        } else {
            registry.validate_shape()?;
        }

        yaml_write(&registry_path, &registry)?;

        Ok(Reply::ok(Out(AddBody {
            registry,
            path,
            added,
        })))
    }
}

/// Success envelope for `registry add`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddBody {
    /// The registry as persisted after the add.
    pub registry: Registry,
    /// Display path of `registry.yaml`.
    pub path: String,
    /// The appended project entry.
    pub added: RegistryProject,
}

impl Render for AddBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "Added `{}` to {}", self.added.name, self.path)?;
        writeln!(w, "registry now declares {} project(s)", self.registry.projects.len())
    }
}

// ---------------------------------------------------------------------------
// registry remove
// ---------------------------------------------------------------------------

/// Wire input for `registry remove`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveInput {
    /// Kebab-case project name to remove.
    pub name: String,
}

/// `specify registry remove <name>` — remove an existing project
/// entry. Warns when `plan.yaml` references it.
#[derive(Debug)]
pub struct Remove {
    input: RemoveInput,
}

impl<P: Anchor> Handler<P> for Remove {
    type Error = crate::verb::Error;
    type Input = RemoveInput;
    type Output = Out<RemoveBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let name = self.input.name;
        let path_buf = Registry::path(&cx.project_dir);
        let path = path_buf.display().to_string();
        let workspace_mode = cx.config.workspace;

        // Pre-flight: surface the `registry-remove-no-registry`
        // diagnostic when the file is absent. `with_state` would
        // emit the generic `Error::ArtifactNotFound`; the registry-specific
        // diag is part of the wire contract.
        if !path_buf.exists() {
            return Err(Error::Diag {
                code: "registry-remove-no-registry",
                detail: format!("registry remove: no registry declared at {path}"),
            }
            .into());
        }

        let project_dir = cx.project_dir.clone();
        let body = with_state::<Registry, _, _>(cx.layout(), "registry.yaml", move |registry| {
            let position =
                registry.projects.iter().position(|p| p.name == name).ok_or_else(|| {
                    Error::Diag {
                        code: "registry-remove-not-found",
                        detail: format!("registry remove: project `{name}` not found in {path}"),
                    }
                })?;
            registry.projects.remove(position);

            // A removal can only relax the multi-repo description
            // invariant, so the post-write check should always
            // succeed; we run it anyway to pin the contract.
            if workspace_mode {
                registry.validate_shape_workspace()?;
            } else {
                registry.validate_shape()?;
            }

            let warnings = plan_refs(&project_dir, &name);
            Ok(RemoveBody {
                registry: registry.clone(),
                path,
                removed: name,
                warnings,
            })
        })?;

        Ok(Reply::ok(Out(body)))
    }
}

/// Success envelope for `registry remove`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveBody {
    /// The registry as persisted after the remove.
    pub registry: Registry,
    /// Display path of `registry.yaml`.
    pub path: String,
    /// The removed project name.
    pub removed: String,
    /// Advisory warnings (stale `plan.yaml` references).
    pub warnings: Vec<String>,
}

impl Render for RemoveBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "Removed `{}` from {}", self.removed, self.path)?;
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
