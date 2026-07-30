//! `emery init` — the project initialization operation.
//!
//! Owns every filesystem write of project-scoped state — `.emery/`,
//! `project.yaml`, `registry.yaml` (workspace mode), `.gitignore`
//! lines, the per-project derived component-mirror cache tenant, and
//! the generated `AGENTS.md` context (plus its `.emery/context.lock`
//! sidecar) when `AGENTS.md` is absent. `--upgrade` is the re-entry
//! path: it bumps the `project.yaml.emery` pin over an existing
//! project, preserving every operator artifact. Running plain `init`
//! in an already-initialized project changes nothing and exits 0 with
//! a message routing to `emery init --upgrade`.
//!
//! Unlike the project-scoped verbs, init runs *before* a project
//! exists, so it anchors at the provider's raw
//! [`Anchor::project_root`] instead of loading `crate::handler::Ctx`.

use std::io::Write;
use std::path::Path;

use error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use super::{EnsuredAdapter, InitOptions, InitResult, init};
use crate::adapter::{AdapterSelector, ComponentMeta, Resolver};
use crate::config::{Layout, ProjectConfig};
use crate::handler::{Anchor, ExecutionPaths, Render};
use crate::platform::parse_platforms_csv;

/// Wire input for `emery init` — the full argument surface.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct InitInput {
    /// Adapter identifier recorded on `project.yaml.adapter`.
    #[serde(default)]
    pub adapter: Option<String>,
    /// Project name override.
    #[serde(default)]
    pub name: Option<String>,
    /// Project description.
    #[serde(default)]
    pub description: Option<String>,
    /// Scaffold a registry-only workspace.
    #[serde(default)]
    pub workspace: bool,
    /// Raw `--platforms` CSV.
    #[serde(default)]
    pub platforms: Option<String>,
    /// Run the re-entry upgrade path over an existing project.
    #[serde(default)]
    pub upgrade: bool,
}

/// `emery init` against the provider's anchor root (`"."` on both
/// sides: the guest's mount preopen, the native process CWD).
#[derive(Clone, Copy, Debug)]
pub struct Init;

impl<P: Anchor + Resolver> Operation<P> for Init {
    type Error = crate::handler::Error;
    type Input = InitInput;
    type Output = InitBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let InitInput {
            adapter,
            name,
            description,
            workspace,
            platforms,
            upgrade,
        } = input;
        let paths = context.provider.paths();
        let project_dir = paths.project_root();

        // Re-entry: an already-initialized project is a no-op that
        // routes the operator to `--upgrade` (docs/init.md §Re-entry).
        if !upgrade && Layout::new(project_dir).config_path().exists() {
            return Ok(InitBody::reentry(project_dir, paths)?);
        }

        if !upgrade && workspace && adapter.is_some() {
            return Err(Error::Diag {
                code: "init-requires-adapter-or-workspace",
                detail: "pass <adapter> or --workspace".to_string(),
            }
            .into());
        }
        if adapter.is_none() && !workspace && !upgrade {
            return Err(Error::validation_failed(
                "init-adapter-required",
                "emery init requires an adapter",
                "pass `<adapter>` (first-party shorthand, package reference, or local component \
                 path), or `--workspace` for a registry-only workspace",
            )
            .into());
        }

        let parsed_platforms =
            platforms.as_deref().map(parse_platforms_csv).transpose().map_err(|e| {
                Error::Argument {
                    flag: "--platforms",
                    detail: e,
                }
            })?;

        // Ensure the adapter binding ahead of the scaffold: fresh init
        // ensures the requested `<adapter>` argument; `--upgrade`
        // re-ensures the project's recorded binding (without rewriting
        // it). Local-component mirroring is the provider's ensure
        // policy; package installation is host-owned (the deployment
        // resolver pulls a missing pin during metadata dispatch).
        let binding =
            if upgrade { ProjectConfig::load(project_dir)?.adapter } else { adapter.clone() };
        let ensured = match binding.as_deref() {
            Some(value) => Some(ensure(context.provider, value, paths).await?),
            None => None,
        };

        let opts = InitOptions {
            project_dir,
            paths,
            adapter: ensured.as_ref(),
            name: name.as_deref(),
            description: description.as_deref(),
            workspace,
            platforms: parsed_platforms.as_deref(),
            upgrade,
        };
        let result = init(context.provider, opts)?;
        let mode = if upgrade { InitMode::Upgraded } else { InitMode::Scaffolded };
        Ok(InitBody::from_result(&result, mode))
    }
}

/// Parse, widen, and ensure one adapter binding through the provider's
/// deployment policy: `expand` pins a bare cache-miss name to the
/// embedded first-party adapter train (identity on deployments with
/// nothing to widen), then ensure provisions it (local-component
/// mirroring; a package pin is installed host-side during the resolve
/// dispatch). The returned selector is the *effective* one — init
/// persists it, so the record names what was ensured.
async fn ensure(
    provider: &impl Resolver, value: &str, paths: &ExecutionPaths,
) -> Result<EnsuredAdapter, Error> {
    let parsed = AdapterSelector::parse(value)?;
    let selector = provider.expand(&parsed, paths);
    let resolved = provider.ensure_target(&selector, paths).await?;
    Ok(EnsuredAdapter { selector, resolved })
}

/// Display a path as the canonical absolute form when it exists; fall
/// back to the lossy display when it does not.
fn canonical(p: &Path) -> String {
    std::fs::canonicalize(p).map_or_else(|_| p.display().to_string(), |c| c.display().to_string())
}

/// Best-effort display name for a recorded adapter value — the
/// re-entry body never fails over a malformed historical value.
fn recorded_name(value: &str) -> String {
    AdapterSelector::parse(value).ok().and_then(|selector| selector.name().ok()).unwrap_or_default()
}

/// Closed outcome discriminant on [`InitBody::mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InitMode {
    /// A fresh scaffold ran.
    Scaffolded,
    /// The project was already initialized; nothing changed.
    AlreadyInitialized,
    /// The `--upgrade` re-entry path ran.
    Upgraded,
}

/// Success envelope for `emery init`.
#[expect(
    clippy::struct_excessive_bools,
    reason = "wire envelope of independent boolean facts; the JSON shape is contract-locked"
)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct InitBody {
    /// What this run did.
    pub mode: InitMode,
    /// Display path of the written `project.yaml`.
    pub config_path: String,
    /// Resolved adapter name (or `"workspace"` for workspace init).
    pub adapter_name: String,
    /// The binding value recorded on `project.yaml.adapter` — the
    /// effective selector after any train expansion (e.g.
    /// `emery:omnia@0.7.0` for a bare cache-miss `omnia`). Absent for
    /// workspace init and the no-op re-entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_binding: Option<String>,
    /// `true` when `--upgrade` rewrote a drifted recorded binding to
    /// the effective ensured selector.
    pub adapter_binding_rewritten: bool,
    /// Whether the project component cache tenant already existed.
    pub cache_present: bool,
    /// Directories the scaffold created.
    pub directories_created: Vec<String>,
    /// Rule keys scaffolded into the project.
    pub scaffolded_rule_keys: Vec<String>,
    /// The `emery` version pinned on `project.yaml`.
    pub emery_version: String,
    /// `true` when this run generated root `AGENTS.md` and
    /// `.emery/context.lock`.
    pub context_generated: bool,
    /// `true` when context generation was skipped.
    pub context_skipped: bool,
    /// Why context generation was skipped (`existing-agents-md` /
    /// `workspace-clone`); absent when it ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_skip_reason: Option<&'static str>,
}

impl InitBody {
    fn from_result(result: &InitResult, mode: InitMode) -> Self {
        Self {
            mode,
            config_path: canonical(&result.config_path),
            adapter_name: result.adapter_name.clone(),
            adapter_binding: result.adapter_binding.clone(),
            adapter_binding_rewritten: result.adapter_binding_rewritten,
            cache_present: result.cache_present,
            directories_created: result.directories_created.iter().map(|p| canonical(p)).collect(),
            scaffolded_rule_keys: result.scaffolded_rule_keys.clone(),
            emery_version: result.emery_version.clone(),
            context_generated: result.context_skip_reason.is_none(),
            context_skipped: result.context_skip_reason.is_some(),
            context_skip_reason: result.context_skip_reason.map(super::Skip::as_str),
        }
    }

    /// The no-op re-entry body, read from the existing `project.yaml`.
    fn reentry(project_dir: &Path, paths: &ExecutionPaths) -> Result<Self, Error> {
        let cfg = ProjectConfig::load(project_dir)?;
        let adapter_name = if cfg.workspace {
            "workspace".to_string()
        } else {
            cfg.adapter.as_deref().map_or_else(String::new, recorded_name)
        };
        Ok(Self {
            mode: InitMode::AlreadyInitialized,
            config_path: canonical(&Layout::new(project_dir).config_path()),
            cache_present: !cfg.workspace && ComponentMeta::path(paths, &adapter_name).exists(),
            adapter_name,
            adapter_binding: None,
            adapter_binding_rewritten: false,
            directories_created: Vec::new(),
            scaffolded_rule_keys: Vec::new(),
            emery_version: cfg.emery_version.unwrap_or_default(),
            context_generated: false,
            context_skipped: false,
            context_skip_reason: None,
        })
    }
}

impl Render for InitBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        match self.mode {
            InitMode::AlreadyInitialized => {
                writeln!(w, "Already initialized ({}); nothing changed.", self.config_path)?;
                writeln!(w, "Run `emery init --upgrade` to bump the emery pin.")?;
                return Ok(());
            }
            InitMode::Upgraded => {
                writeln!(w, "Upgraded .emery/")?;
            }
            InitMode::Scaffolded if self.adapter_name == "workspace" => {
                writeln!(w, "Scaffolded .emery/ as a registry-only workspace")?;
            }
            InitMode::Scaffolded => {
                writeln!(w, "Scaffolded .emery/")?;
            }
        }
        match &self.adapter_binding {
            Some(binding) => writeln!(w, "  adapter: {binding}")?,
            None => writeln!(w, "  adapter: {}", self.adapter_name)?,
        }
        if self.adapter_binding_rewritten
            && let Some(binding) = &self.adapter_binding
        {
            writeln!(w, "  adapter binding rewritten to {binding}")?;
        }
        writeln!(w, "  config: {}", self.config_path)?;
        writeln!(w, "  emery: {}", self.emery_version)?;
        if self.context_skip_reason == Some("existing-agents-md") {
            writeln!(w, "AGENTS.md already present; skipping context generate")?;
        }
        Ok(())
    }
}
