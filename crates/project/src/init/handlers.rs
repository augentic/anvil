//! `specify init` — the project initialization operation.
//!
//! Owns every filesystem write of project-scoped state — `.specify/`,
//! `project.yaml`, `registry.yaml` (workspace mode), `.gitignore`
//! lines, the per-project derived component-mirror cache tenant, and
//! the generated `AGENTS.md` context (plus its `.specify/context.lock`
//! sidecar) when `AGENTS.md` is absent. `--upgrade` is the re-entry
//! path: it bumps the `project.yaml.specify` pin over an existing
//! project, preserving every operator artifact. Running plain `init`
//! in an already-initialized project changes nothing and exits 0 with
//! a message routing to `specify init --upgrade`.
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

/// Wire input for `specify init` — the full argument surface.
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

/// `specify init` against the provider's anchor root (`"."` on both
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
                "specify init requires an adapter",
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
        // it). Package hydration into the global store, digest
        // verification, and local-component mirroring are the
        // provider's ensure policy.
        let binding =
            if upgrade { ProjectConfig::load(project_dir)?.adapter } else { adapter.clone() };
        let mut hydrated = Vec::new();
        let ensured = match binding.as_deref() {
            Some(value) => Some(ensure(context.provider, value, paths, &mut hydrated).await?),
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
        Ok(InitBody::from_result(&result, mode, hydrated))
    }
}

/// Parse and ensure one adapter binding, recording any package pin
/// this run installed into the global store on `hydrated` (the store
/// state is observed around the ensure so the envelope reports only
/// actual fetches).
async fn ensure(
    provider: &impl Resolver, value: &str, paths: &ExecutionPaths, hydrated: &mut Vec<String>,
) -> Result<EnsuredAdapter, Error> {
    let selector = AdapterSelector::parse(value)?;
    let pin = match &selector {
        AdapterSelector::Package { name, version, .. } => {
            let installed =
                diagnostics::cache::adapter_store_entry(name, &version.to_string()).is_file();
            (!installed).then(|| format!("{name}@{version}"))
        }
        AdapterSelector::Bare { .. } | AdapterSelector::Component { .. } => None,
    };
    let resolved = provider.ensure_target(&selector, paths).await?;
    if let Some(identity) = pin {
        hydrated.push(identity);
    }
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

/// Success envelope for `specify init`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "wire DTO — each flag is a documented JSON field on the init envelope"
)]
pub struct InitBody {
    /// What this run did.
    pub mode: InitMode,
    /// Display path of the written `project.yaml`.
    pub config_path: String,
    /// Resolved adapter name (or `"workspace"` for workspace init).
    pub adapter_name: String,
    /// Whether the project component cache tenant already existed.
    pub cache_present: bool,
    /// Directories the scaffold created.
    pub directories_created: Vec<String>,
    /// Rule keys scaffolded into the project.
    pub scaffolded_rule_keys: Vec<String>,
    /// The `specify` version pinned on `project.yaml`.
    pub specify_version: String,
    /// Pinned identities (`<name>@<version>`) this run fetched into
    /// the global adapter store.
    pub hydrated: Vec<String>,
    /// Whether a wasm-pkg config was written.
    pub wasm_pkg_config_written: bool,
    /// `true` when this run generated root `AGENTS.md` and
    /// `.specify/context.lock`.
    pub context_generated: bool,
    /// `true` when context generation was skipped.
    pub context_skipped: bool,
    /// Why context generation was skipped (`existing-agents-md` /
    /// `workspace-clone`); absent when it ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_skip_reason: Option<&'static str>,
}

impl InitBody {
    fn from_result(result: &InitResult, mode: InitMode, hydrated: Vec<String>) -> Self {
        Self {
            mode,
            config_path: canonical(&result.config_path),
            adapter_name: result.adapter_name.clone(),
            cache_present: result.cache_present,
            directories_created: result.directories_created.iter().map(|p| canonical(p)).collect(),
            scaffolded_rule_keys: result.scaffolded_rule_keys.clone(),
            specify_version: result.specify_version.clone(),
            hydrated,
            wasm_pkg_config_written: result.wasm_pkg_config_written,
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
            adapter_name,
            cache_present: ComponentMeta::path(paths).exists(),
            directories_created: Vec::new(),
            scaffolded_rule_keys: Vec::new(),
            specify_version: cfg.specify_version.unwrap_or_default(),
            hydrated: Vec::new(),
            wasm_pkg_config_written: false,
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
                writeln!(w, "Run `specify init --upgrade` to bump the specify pin.")?;
                return Ok(());
            }
            InitMode::Upgraded => {
                writeln!(w, "Upgraded .specify/")?;
            }
            InitMode::Scaffolded if self.adapter_name == "workspace" => {
                writeln!(w, "Scaffolded .specify/ as a registry-only workspace")?;
            }
            InitMode::Scaffolded => {
                writeln!(w, "Scaffolded .specify/")?;
            }
        }
        writeln!(w, "  adapter: {}", self.adapter_name)?;
        writeln!(w, "  config: {}", self.config_path)?;
        writeln!(w, "  specify: {}", self.specify_version)?;
        for identity in &self.hydrated {
            writeln!(w, "  hydrated: {identity}")?;
        }
        if self.context_skip_reason == Some("existing-agents-md") {
            writeln!(w, "AGENTS.md already present; skipping context generate")?;
        }
        Ok(())
    }
}
