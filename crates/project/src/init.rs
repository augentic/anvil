//! Orchestration for `emery init`. Scaffolds `.emery/`, resolves
//! the requested adapter, writes `project.yaml`, and upserts
//! `.gitignore` lines.

mod context;
mod gitignore;
pub mod handlers;
mod regular;
mod upgrade;

use std::path::{Path, PathBuf};

use context::Skip;
use error::Error;

use crate::adapter::{AdapterSelector, PlatformsSurface, ResolvedTarget};
use crate::handler::ExecutionPaths;
use crate::platform::Platform;

/// The adapter binding an init run ensured ahead of the scaffold: the
/// operator's selector as typed plus the deployment's resolved
/// identity. Provisioning (mirroring; host-owned package install)
/// already happened through
/// [`crate::adapter::Resolver::ensure_target`].
#[derive(Debug, Clone)]
pub(crate) struct EnsuredAdapter {
    /// The selector as parsed from the `<adapter>` argument (fresh
    /// init) or the recorded `project.yaml.adapter` (`--upgrade`).
    pub selector: AdapterSelector,
    /// The ensured, resolved target adapter.
    pub resolved: ResolvedTarget,
}

/// Inputs to [`init`].
///
/// Borrow-shaped so callers (the CLI and tests) can build the struct
/// without cloning path buffers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InitOptions<'a> {
    pub project_dir: &'a Path,
    /// The provider's execution paths (cache placement) for the
    /// component-mirror tenant probes.
    pub paths: &'a ExecutionPaths,
    /// The ensured adapter binding. Required for a fresh init.
    pub adapter: Option<&'a EnsuredAdapter>,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    /// Target platforms to declare in `project.yaml`. Parsed from the
    /// `--platforms` CLI flag (comma-separated). `None` means the
    /// operator did not pass `--platforms`. When the resolved target
    /// adapter declares `platforms.required`, this must be `Some`.
    pub platforms: Option<&'a [Platform]>,
    /// When `true`, run the re-entry **upgrade** path instead of a
    /// fresh scaffold: bump `project.yaml.emery` to the running
    /// binary's version over an already-populated `.emery/`, preserving
    /// every other field and every operator artifact. Mutually
    /// exclusive with the `<adapter>` positional, `--name`, and
    /// `--description`; `--platforms` stays legal.
    pub upgrade: bool,
}

/// Structured summary of what `init` did, returned for downstream
/// rendering by both the JSON and text CLI paths.
#[derive(Debug, Clone)]
pub(crate) struct InitResult {
    pub config_path: PathBuf,
    /// Resolved adapter name from the adapter root.
    pub adapter_name: String,
    /// The binding value recorded on `project.yaml.adapter` — the
    /// selector as typed (a bare name stays bare).
    pub adapter_binding: Option<String>,
    pub cache_present: bool,
    pub directories_created: Vec<PathBuf>,
    pub scaffolded_rule_keys: Vec<String>,
    pub emery_version: String,
    /// Why init-time context generation was skipped; `None` when this
    /// run generated root `AGENTS.md` and `.emery/context.lock`.
    pub context_skip_reason: Option<Skip>,
}

/// Initialise `.emery/` inside `opts.project_dir`.
///
/// Idempotent: a second call with identical options succeeds, creates no
/// new directories, doesn't duplicate the `.gitignore` entry, and writes
/// byte-identical `project.yaml` contents. [`InitOptions::upgrade`]
/// dispatches to its private runner ahead of the fresh scaffold.
///
/// # Errors
///
/// A fresh init requires [`InitOptions::adapter`]
/// (`init-adapter-required`). Bubbles up filesystem, adapter
/// resolution, and serialisation errors.
pub(crate) fn init(
    resolver: &impl crate::adapter::Resolver, opts: InitOptions<'_>,
) -> Result<InitResult, Error> {
    let mut result = if opts.upgrade { upgrade::run(opts)? } else { regular::run(opts)? };
    // Every branch shares one context-generation pass over the freshly
    // written project: the skip logic (existing `AGENTS.md`) gives
    // `--upgrade` its regenerate-only-when-absent behavior.
    result.context_skip_reason = context::generate(resolver, opts.paths)?;
    Ok(result)
}

/// The value init records on `project.yaml.adapter` for one ensured
/// binding: the selector as typed (a bare name stays bare).
///
/// A component path is recorded as its canonical `file://` form so the
/// value outlives the CWD — read from the cache mirror's provenance
/// sidecar when present (the guest cannot canonicalize a host path
/// outside its mounts); canonicalized directly otherwise.
pub(crate) fn binding_value(
    ensured: &EnsuredAdapter, paths: &ExecutionPaths, project_dir: &Path,
) -> Result<String, Error> {
    match &ensured.selector {
        AdapterSelector::Component { .. } => {
            crate::adapter::ComponentMeta::load(paths, &ensured.resolved.manifest.name)
                .map_or_else(|| ensured.selector.persist_value(project_dir), |meta| Ok(meta.source))
        }
        _ => ensured.selector.persist_value(project_dir),
    }
}

pub(crate) fn resolved_name(project_dir: &Path, explicit: Option<&str>) -> String {
    if let Some(explicit) = explicit {
        return explicit.to_string();
    }
    project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map_or_else(|| "project".to_string(), str::to_string)
}

pub(crate) fn resolve_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub(crate) fn upsert_gitignore(project_dir: &Path) -> Result<(), Error> {
    gitignore::ensure_gitignore(project_dir)
}

/// Validate the operator's `--platforms` set against the target's
/// declared capability, mapping each violation onto the init-time
/// `project-platforms-*` diagnostic family via the shared
/// platform-validation error converter. The rules themselves live on
/// [`crate::adapter::PlatformsCapability::check`].
pub(crate) fn validate_platforms(
    operator: Option<&[Platform]>, capability: Option<&crate::adapter::PlatformsCapability>,
    target_name: &str,
) -> Result<Vec<Platform>, Error> {
    let platforms = operator.map(<[Platform]>::to_vec).unwrap_or_default();
    let Some(cap) = capability else {
        return Ok(platforms);
    };

    cap.check(&platforms).map_err(|violation| {
        violation.into_error(PlatformsSurface::Init { target: target_name })
    })?;

    Ok(platforms)
}
