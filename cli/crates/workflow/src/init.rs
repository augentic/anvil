//! Orchestration for `specify init`. Scaffolds `.specify/`, resolves
//! the requested adapter, writes `project.yaml`, and upserts
//! `.gitignore` lines. Workspace mode additionally mints `registry.yaml`.

mod adapter_uri;
mod cache;
mod git;
mod regular;
mod upgrade;
mod workspace;

use std::path::{Path, PathBuf};

pub use adapter_uri::{
    AdapterPackage, adapter_name_from_value, adapter_ref_from_value, recognize_package,
};
pub use cache::{CodexMeta, codex_cache_root};
use jiff::Timestamp;
use specify_error::Error;
use specify_extension_manifest::{DEFAULT_WASM_PKG_CONFIG, WASM_PKG_CONFIG_FILENAME};
use specify_model::atomic::bytes_write;

use crate::adapter::PlatformsViolation;
use crate::config::Layout;
use crate::platform::Platform;

/// Inputs to [`init`].
///
/// Borrow-shaped so callers (the CLI and tests) can build the struct
/// without cloning path buffers. All fields are `Copy` references or
/// scalars, so the struct is `Copy` and threads through the workspace /
/// regular runners by value without a clone.
#[derive(Debug, Clone, Copy)]
pub struct InitOptions<'a> {
    /// Root of the project being initialised.
    pub project_dir: &'a Path,
    /// Adapter identifier (bare name like `omnia` or a URL) to fetch
    /// or copy into the out-of-tree per-project cache. Required for regular init; must
    /// be `None` when [`InitOptions::workspace`] is `true` (workspace
    /// roots do not resolve an adapter at init time).
    pub adapter: Option<&'a str>,
    /// Project name; defaults to the project directory name when `None`.
    pub name: Option<&'a str>,
    /// Optional free-text project description (tech stack, architecture,
    /// testing approach).
    pub description: Option<&'a str>,
    /// When `true`, scaffold a registry-only **workspace** instead
    /// of a regular project: writes `registry.yaml` at the repo root
    /// and `project.yaml { workspace: true }` (with `adapter:` omitted)
    /// under `.specify/`. Workspace init refuses to run when `.specify/`
    /// already exists so it never clobbers a regular single-repo project.
    pub workspace: bool,
    /// When `true`, also distribute the framework `core/` pack
    /// (`adapters/shared/rules/core/`) into the project codex cache
    /// alongside the always-distributed `universal/` pack. Default off:
    /// consumer projects carry only `UNI-*` rules. Ignored for workspace
    /// init (workspaces resolve no adapter and so distribute no codex).
    pub include_framework: bool,
    /// Target platforms to declare in `project.yaml`. Parsed from the
    /// `--platforms` CLI flag (comma-separated). `None` means the
    /// operator did not pass `--platforms`. When the resolved target
    /// adapter declares `platforms.required`, this must be `Some`.
    pub platforms: Option<&'a [Platform]>,
    /// When `true`, run the re-entry **upgrade** path instead of a
    /// fresh scaffold: bump `project.yaml.specify_version` to the
    /// running binary's version over an already-populated `.specify/`,
    /// preserving every other field (including `adapter:` / `workspace:`)
    /// and every operator artifact (`slices/`, `specs/`, `archive/`,
    /// `registry.yaml`, `.specify/design-system/*`, the adapter cache).
    /// `AGENTS.md` is regenerated only when absent (handled at the
    /// command layer). Mutually exclusive with the `<adapter>`
    /// positional, `--workspace`, `--name`, `--description`, and
    /// `--include-framework` at the clap surface. `--platforms` is
    /// legal alongside `--upgrade`.
    pub upgrade: bool,
}

/// Structured summary of what `init` did, returned for downstream
/// rendering by both the JSON and text CLI paths.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent on-disk fact the renderer surfaces separately on the init result."
)]
pub struct InitResult {
    /// Path to the written `project.yaml`.
    pub config_path: PathBuf,
    /// Resolved adapter name from the adapter root. For workspace init
    /// this is the literal `"workspace"` so the JSON envelope stays stable
    /// for downstream consumers.
    pub adapter_name: String,
    /// Whether `manifests/manifest-meta.yaml` exists in the per-project cache.
    pub cache_present: bool,
    /// Whether the shared codex was distributed into the out-of-tree
    /// `<project-cache>/codex/` during this run. `false` when the
    /// adapter source tree carries no `adapters/shared/rules/universal/`
    /// pack (the consumer then relies on `--rules-root` or a monorepo
    /// checkout) and for workspace init.
    pub codex_present: bool,
    /// Directories that were newly created (empty on re-init).
    pub directories_created: Vec<PathBuf>,
    /// Brief IDs scaffolded into the `rules:` map.
    pub scaffolded_rule_keys: Vec<String>,
    /// The `specify_version` value recorded in `project.yaml` after
    /// this run (the running binary's version).
    pub specify_version: String,
    /// `true` when this run actually wrote `project.yaml.specify_version`
    /// — always `true` for fresh init (the file is minted) and for an
    /// `--upgrade` that bumped an older pin; `false` only on an
    /// `--upgrade` no-op where the pin already matched the running
    /// version. Lets the renderer distinguish "upgraded" from "already
    /// current".
    pub specify_version_changed: bool,
    /// `true` when this run wrote `.specify/wasm-pkg.toml` for the
    /// first time; `false` when an operator-edited file was preserved.
    /// Lets the JSON envelope distinguish a fresh scaffold from a
    /// re-init that left registry config intact.
    pub wasm_pkg_config_written: bool,
}

/// Initialise `.specify/` inside `opts.project_dir`.
///
/// Idempotent: a second call with identical options succeeds, creates no
/// new directories, doesn't duplicate the `.gitignore` entry, and writes
/// byte-identical `project.yaml` contents.
///
/// When [`InitOptions::upgrade`] is `true`, dispatches to the private
/// upgrade runner (the re-entry version bump) ahead of the workspace /
/// regular branch — one runner serves both regular and workspace
/// projects because the preservation logic is identical (preserve every
/// field, touch only `specify_version`).
///
/// When [`InitOptions::workspace`] is `true`, dispatches to the private
/// workspace runner for the workspace on-disk shape.
///
/// `now` records the `manifest-meta.yaml::fetched_at` stamp; the dispatcher
/// passes `Timestamp::now` and tests pin a deterministic value.
///
/// # Errors
///
/// Pre-condition: regular (non-workspace) init requires
/// [`InitOptions::adapter`] to be set; the CLI dispatcher enforces
/// the `init-requires-adapter-or-workspace` invariant ahead of this call,
/// but `init` re-validates as a defence in depth. Bubbles up
/// filesystem, adapter resolution, and serialisation errors from
/// the underlying calls.
pub fn init(opts: InitOptions<'_>, now: Timestamp) -> Result<InitResult, Error> {
    if opts.upgrade {
        return upgrade::run(opts);
    }
    if opts.workspace {
        return workspace::run(opts);
    }
    regular::run(opts, now)
}

/// Distribute (or refresh) the shared codex for an initialised project.
///
/// Pinned to `adapter_value` — the project's recorded `adapter:`
/// source/ref (or an operator override). Resolves the adapter source
/// the same way `init` does (local copy or git sparse checkout), then
/// mirrors `adapters/shared/rules/universal/` (and, when
/// `include_framework`, `core/`) into the out-of-tree `<project-cache>/codex/`.
///
/// This is the engine behind `specify rules sync`. `init` distributes
/// the codex inline via the private `cache::cache_codex` path; this
/// entry point lets a refresh run stand alone without re-running init.
///
/// Returns `Ok(true)` when the codex was distributed, `Ok(false)` when
/// the adapter source carries no `adapters/shared/rules/universal/`
/// pack (fail-soft). `now` stamps [`CodexMeta::fetched_at`].
///
/// # Errors
///
/// Bubbles up adapter-resolution (clone/copy) and filesystem errors.
pub fn sync_codex(
    project_dir: &Path, adapter_value: &str, include_framework: bool, now: Timestamp,
) -> Result<bool, Error> {
    let source = adapter_uri::AdapterUri::parse(adapter_value, project_dir)?;
    cache::cache_codex(project_dir, &source, include_framework, now)
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
    crate::registry::ensure_gitignore_entries(project_dir)
}

/// Scaffold the project-local wasm-pkg config when absent, preserving
/// any operator-edited file byte-for-byte on re-init.
///
/// The contents are the canonical wasm-pkg namespace mapping
/// (`specify -> augentic.io`); see
/// [`specify_extension_manifest::DEFAULT_WASM_PKG_CONFIG`]. Operators are expected
/// to edit this file to add private mirrors or other namespace
/// mappings, so a re-init must never clobber their changes.
///
/// Returns `Ok(true)` when this call wrote the file, `Ok(false)` when
/// it already existed.
///
/// # Errors
///
/// Propagates filesystem errors from creating `.specify/` or writing
/// the file.
pub(crate) fn scaffold_wasm_pkg_config(layout: &Layout<'_>) -> Result<bool, Error> {
    let path = layout.specify_dir().join(WASM_PKG_CONFIG_FILENAME);
    if path.exists() {
        return Ok(false);
    }
    bytes_write(&path, DEFAULT_WASM_PKG_CONFIG.as_bytes())?;
    Ok(true)
}

/// Validate the operator's `--platforms` set against the target's
/// declared capability, mapping each [`PlatformsViolation`] onto the
/// init-time `project-platforms-*` diagnostic family. The rules
/// themselves live on [`crate::adapter::PlatformsCapability::check`].
pub(crate) fn validate_platforms(
    operator: Option<&[Platform]>, capability: Option<&crate::adapter::PlatformsCapability>,
    target_name: &str,
) -> Result<Vec<Platform>, Error> {
    let platforms = operator.map(<[Platform]>::to_vec).unwrap_or_default();
    let Some(cap) = capability else {
        return Ok(platforms);
    };

    cap.check(&platforms).map_err(|violation| match violation {
        PlatformsViolation::RequiredButMissing { defaults } => Error::validation_failed(
            "project-platforms-required",
            format!("target '{target_name}' requires --platforms"),
            format!(
                "target '{target_name}' requires --platforms; default set is [{}]",
                defaults.join(", "),
            ),
        ),
        PlatformsViolation::MissingCore => Error::validation_failed(
            "project-platforms-must-include-core",
            "platform set must include `core`",
            "the --platforms set must include `core`; every project that declares platforms requires the shared Rust core crate",
        ),
        PlatformsViolation::NotAllowed { platform, allowed } => Error::validation_failed(
            "project-platforms-not-allowed",
            format!("platform `{platform}` is not in the target's allowed set"),
            format!(
                "platform `{platform}` is not allowed by target '{target_name}'; allowed: [{}]",
                allowed.join(", "),
            ),
        ),
    })?;

    Ok(platforms)
}

#[cfg(test)]
pub(super) fn fixed_now() -> Timestamp {
    "2026-05-07T00:00:00Z".parse().expect("fixed test stamp")
}

#[cfg(test)]
mod tests;
