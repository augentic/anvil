//! `emery init` (ADR-0009 §1): resolve each requested source adapter
//! on the source axis, record the authored bindings on
//! `project.yaml`, and scaffold `.emery/`.

use std::io::Write;
use std::path::{Path, PathBuf};

use emery_error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, ExecutionPaths, Render};
use crate::project::{BindingContent, Project, SourceBinding};
use crate::resolve::{AdapterSelector, ComponentMeta, Resolver};

/// Wire input for `emery init` — the full argument surface.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct InitInput {
    /// Source adapters to bind as workspace-backed sources.
    #[serde(default)]
    pub adapters: Vec<String>,
    /// Value-backed source bindings, each `<adapter>=<text>`.
    #[serde(default)]
    pub values: Vec<String>,
    /// Project name override.
    #[serde(default)]
    pub name: Option<String>,
    /// Project description.
    #[serde(default)]
    pub description: Option<String>,
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
            adapters,
            values,
            name,
            description,
            upgrade,
        } = input;
        let paths = context.provider.paths();
        let project_dir = paths.project_root();

        if upgrade {
            return Ok(run_upgrade(context.provider, project_dir, paths).await?);
        }

        // Re-entry: an already-initialized project is a no-op that
        // routes the operator to `--upgrade`.
        if Project::path(project_dir).exists() {
            let project = Project::load(project_dir)?;
            return Ok(InitBody::from_project(InitMode::AlreadyInitialized, &project, project_dir));
        }

        if adapters.is_empty() && values.is_empty() {
            return Err(Error::validation_failed(
                "init-source-required",
                "emery init requires at least one source adapter",
                "pass `<adapter>` (package reference or local component path) and/or `--value \
                 <adapter>=<text>` for an inline source",
            )
            .into());
        }

        let mut sources = Vec::new();
        for value in &adapters {
            let bound =
                bind(context.provider, value, paths, BindingContent::Workspace(".".to_string()))
                    .await?;
            push_unique(&mut sources, bound)?;
        }
        for entry in &values {
            let (adapter, text) = split_value(entry)?;
            let bound =
                bind(context.provider, adapter, paths, BindingContent::Value(text.to_string()))
                    .await?;
            push_unique(&mut sources, bound)?;
        }

        let project = Project {
            name: resolved_name(project_dir, name.as_deref()),
            description,
            emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            sources,
        };
        std::fs::create_dir_all(project_dir.join(".emery")).map_err(Error::Io)?;
        project.store(project_dir)?;
        Ok(InitBody::from_project(InitMode::Scaffolded, &project, project_dir))
    }
}

/// The `--upgrade` re-entry: re-ensure every recorded binding and bump
/// the `emery` pin, preserving everything else.
async fn run_upgrade(
    provider: &impl Resolver, project_dir: &Path, paths: &ExecutionPaths,
) -> Result<InitBody, Error> {
    let mut project = Project::load(project_dir)?;
    for binding in &project.sources {
        let selector = AdapterSelector::parse(&binding.adapter)?;
        provider.ensure_source(&selector, paths).await?;
    }
    project.emery_version = Some(env!("CARGO_PKG_VERSION").to_string());
    project.store(project_dir)?;
    Ok(InitBody::from_project(InitMode::Upgraded, &project, project_dir))
}

/// Ensure one adapter on the source axis and shape its binding: the
/// key is the resolved adapter name; the persisted selector value
/// outlives the CWD (a local component records its canonical
/// `file://` form from the cache mirror's provenance sidecar).
async fn bind(
    provider: &impl Resolver, value: &str, paths: &ExecutionPaths, content: BindingContent,
) -> Result<SourceBinding, Error> {
    let selector = AdapterSelector::parse(value)?;
    let resolved = provider.ensure_source(&selector, paths).await?;
    let key = resolved.manifest.name;
    let adapter = match &selector {
        AdapterSelector::Component { .. } => ComponentMeta::load(paths, &key)
            .map_or_else(|| selector.persist_value(paths.project_root()), |meta| Ok(meta.source))?,
        _ => selector.persist_value(paths.project_root())?,
    };
    Ok(SourceBinding {
        key,
        adapter,
        content,
    })
}

/// Append `binding` unless its key is already bound.
fn push_unique(sources: &mut Vec<SourceBinding>, binding: SourceBinding) -> Result<(), Error> {
    if sources.iter().any(|existing| existing.key == binding.key) {
        return Err(Error::validation_failed(
            "init-source-duplicate",
            "each source binds once",
            format!("source `{}` is bound twice", binding.key),
        ));
    }
    sources.push(binding);
    Ok(())
}

/// Split one `--value <adapter>=<text>` entry at the first `=`.
fn split_value(entry: &str) -> Result<(&str, &str), Error> {
    entry.split_once('=').filter(|(adapter, _)| !adapter.is_empty()).ok_or_else(|| {
        Error::Argument {
            flag: "--value",
            detail: format!("expected `<adapter>=<text>`, got `{entry}`"),
        }
    })
}

fn resolved_name(project_dir: &Path, explicit: Option<&str>) -> String {
    if let Some(explicit) = explicit {
        return explicit.to_string();
    }
    project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "project".to_string(), str::to_string)
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
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct InitBody {
    /// What this run did.
    pub mode: InitMode,
    /// Canonical path of the written `project.yaml`.
    pub config_path: PathBuf,
    /// The bound source keys, in binding order.
    pub sources: Vec<String>,
    /// The `emery` version pinned on `project.yaml`.
    pub emery_version: String,
}

impl InitBody {
    fn from_project(mode: InitMode, project: &Project, project_dir: &Path) -> Self {
        let path = Project::path(project_dir);
        Self {
            mode,
            config_path: std::fs::canonicalize(&path).unwrap_or(path),
            sources: project.sources.iter().map(|binding| binding.key.clone()).collect(),
            emery_version: project.emery_version.clone().unwrap_or_default(),
        }
    }
}

impl Render for InitBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        match self.mode {
            InitMode::AlreadyInitialized => {
                writeln!(
                    w,
                    "Already initialized ({}); nothing changed.",
                    self.config_path.display()
                )?;
                writeln!(w, "Run `emery init --upgrade` to bump the emery pin.")?;
                return Ok(());
            }
            InitMode::Upgraded => writeln!(w, "Upgraded .emery/")?,
            InitMode::Scaffolded => writeln!(w, "Scaffolded .emery/")?,
        }
        writeln!(w, "  sources: {}", self.sources.join(", "))?;
        writeln!(w, "  config: {}", self.config_path.display())?;
        writeln!(w, "  emery: {}", self.emery_version)?;
        Ok(())
    }
}
