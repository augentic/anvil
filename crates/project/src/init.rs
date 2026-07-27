//! Orchestration for `emery init`. Scaffolds `.emery/`, resolves
//! the requested adapter, writes `project.yaml`, and upserts
//! `.gitignore` lines. Workspace mode additionally mints `registry.yaml`.

mod context;
pub mod handlers;
mod regular;
mod upgrade;
mod workspace;

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
    /// The ensured adapter binding. Required for regular init; must
    /// be `None` when [`InitOptions::workspace`] is `true` (workspace
    /// roots do not resolve an adapter at init time).
    pub adapter: Option<&'a EnsuredAdapter>,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    /// When `true`, scaffold a registry-only **workspace** instead
    /// of a regular project: writes `registry.yaml` at the repo root
    /// and `project.yaml { workspace: true }` (with `adapter:` omitted)
    /// under `.emery/`. Workspace init refuses to run when `.emery/`
    /// already exists so it never clobbers a regular single-repo project.
    pub workspace: bool,
    /// Target platforms to declare in `project.yaml`. Parsed from the
    /// `--platforms` CLI flag (comma-separated). `None` means the
    /// operator did not pass `--platforms`. When the resolved target
    /// adapter declares `platforms.required`, this must be `Some`.
    pub platforms: Option<&'a [Platform]>,
    /// When `true`, run the re-entry **upgrade** path instead of a
    /// fresh scaffold: bump `project.yaml.emery` to the
    /// running binary's version over an already-populated `.emery/`,
    /// preserving every other field (including `adapter:` / `workspace:`)
    /// and every operator artifact (`slices/`, `specs/`, `archive/`,
    /// `registry.yaml`, `.emery/design-system/*`, the adapter cache).
    /// `AGENTS.md` is regenerated only when absent (handled at the
    /// command layer). Mutually exclusive with the `<adapter>`
    /// positional, `--workspace`, `--name`, and `--description` at the
    /// clap surface. `--platforms` is legal alongside `--upgrade`.
    pub upgrade: bool,
}

/// Structured summary of what `init` did, returned for downstream
/// rendering by both the JSON and text CLI paths.
#[derive(Debug, Clone)]
pub(crate) struct InitResult {
    pub config_path: PathBuf,
    /// Resolved adapter name from the adapter root. For workspace init
    /// this is the literal `"workspace"` so the JSON envelope stays stable
    /// for downstream consumers.
    pub adapter_name: String,
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
/// byte-identical `project.yaml` contents.
///
/// When [`InitOptions::upgrade`] is `true`, dispatches to the private
/// upgrade runner (the re-entry version bump) ahead of the workspace /
/// regular branch — one runner serves both regular and workspace
/// projects because the preservation logic is identical (preserve every
/// field, touch only `emery`).
///
/// When [`InitOptions::workspace`] is `true`, dispatches to the private
/// workspace runner for the workspace on-disk shape.
///
/// # Errors
///
/// Pre-condition: regular (non-workspace) init requires
/// [`InitOptions::adapter`] to be set; the operation layer enforces
/// this ahead of the call (the typed `init-adapter-required`), and
/// `init` re-validates as a defence in depth
/// (`init-requires-adapter-or-workspace`). Bubbles up
/// filesystem, adapter resolution, and serialisation errors from
/// the underlying calls.
pub(crate) fn init(
    resolver: &impl crate::adapter::Resolver, opts: InitOptions<'_>,
) -> Result<InitResult, Error> {
    let mut result = if opts.upgrade {
        upgrade::run(opts)?
    } else if opts.workspace {
        workspace::run(opts)?
    } else {
        regular::run(opts)?
    };
    // Every branch shares one context-generation pass over the freshly
    // written project: the skip logic (existing `AGENTS.md`, workspace
    // slot) gives `--upgrade` its regenerate-only-when-absent behavior.
    result.context_skip_reason = context::generate(resolver, opts.paths)?;
    Ok(result)
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
    crate::registry::ensure_gitignore(project_dir)
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
