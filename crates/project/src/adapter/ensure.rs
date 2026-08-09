//! Component-deployment kernels behind the provider's ensure legs.
//!
//! Only a local component selector provisions here; bare names and
//! package pins provision nothing in-guest.

use std::fs;
use std::path::{Path, PathBuf};

use error::Error;
use serde::{Deserialize, Serialize};

use super::core::{ResolvedSource, ResolvedTarget};
use super::resolver::{Component, Resolver as _, component_cache_entry};
use super::selector::canonicalize_component;
use super::{AdapterSelector, metadata, selector};
use crate::handler::ExecutionPaths;

/// Ensure a source selector for the component deployment: provision
/// (mirror), then resolve through the component resolver.
///
/// # Errors
///
/// Provisioning failures (`adapter-component-missing`,
/// `adapter-canonicalize-failed`) ahead of resolve failures.
pub fn source(
    runner: metadata::Runner, selector: &AdapterSelector, paths: &ExecutionPaths,
    now: jiff::Timestamp,
) -> Result<ResolvedSource, Error> {
    provision(selector, paths, now)?;
    Component::new(runner).resolve_source(selector, paths)
}

/// Ensure a target selector for the component deployment: provision
/// (mirror), then resolve through the component resolver.
///
/// # Errors
///
/// Provisioning failures (`adapter-component-missing`,
/// `adapter-canonicalize-failed`) ahead of resolve failures.
pub fn target(
    runner: metadata::Runner, selector: &AdapterSelector, paths: &ExecutionPaths,
    now: jiff::Timestamp,
) -> Result<ResolvedTarget, Error> {
    provision(selector, paths, now)?;
    Component::new(runner).resolve_target(selector, paths)
}

/// Make one selector resolvable on the guest side of the seam: mirror
/// a local component into the project cache, or nothing for a bare
/// development name or a package pin (host-installed on dispatch).
///
/// # Errors
///
/// `adapter-component-missing` or `adapter-canonicalize-failed`.
pub fn provision(
    selector: &AdapterSelector, paths: &ExecutionPaths, now: jiff::Timestamp,
) -> Result<(), Error> {
    match selector {
        AdapterSelector::Bare { .. } | AdapterSelector::Package { .. } => Ok(()),
        AdapterSelector::Component { path } => mirror(path, paths, now),
    }
}

/// Mirror an operator-supplied local component into the project
/// component cache, stamping provenance in [`ComponentMeta`].
///
/// Carries the persisted-mirror fallback: a component selector stays
/// resolvable after the original file is removed because the earlier
/// mirror satisfies re-ensure. The explicit [`seed`] verb has no such
/// fallback — a missing path there is an operator mistake to surface.
fn mirror(path: &Path, paths: &ExecutionPaths, now: jiff::Timestamp) -> Result<(), Error> {
    let absolute =
        if path.is_absolute() { path.to_path_buf() } else { paths.project_root().join(path) };
    if !absolute.is_file()
        && let Ok(name) = selector::name_from_component(path)
        && component_cache_entry(paths, &name).is_file()
    {
        return Ok(());
    }
    seed(path, paths, now).map(drop)
}

/// The seeded identity one [`seed`] produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seeded {
    /// Kebab-case adapter name derived from the component filename.
    pub name: String,
    /// The mirrored project component cache entry.
    pub entry: PathBuf,
    /// The canonical operator-supplied component the entry mirrors.
    pub source: PathBuf,
}

/// Seed one operator-supplied `.wasm` component into the project
/// component cache.
///
/// Canonicalizes, derives the kebab name from the filename, copies to
/// `<project-cache>/components/<name>.wasm`, and stamps provenance.
/// Re-seeding replaces the entry; a wrong-world component fails at the
/// dispatch gate, not during seeding. Strict: a missing path fails
/// even when the derived name is already cached (no typo masking).
///
/// # Errors
///
/// `adapter-component-missing` when `path` is not a `.wasm` file,
/// `adapter-canonicalize-failed` when it cannot be canonicalized, and
/// I/O failures from the copy or provenance write.
pub fn seed(path: &Path, paths: &ExecutionPaths, now: jiff::Timestamp) -> Result<Seeded, Error> {
    let absolute =
        if path.is_absolute() { path.to_path_buf() } else { paths.project_root().join(path) };
    ensure_component_file(&absolute, &path.display().to_string())?;
    let canonical = canonicalize_component(path, paths.project_root())?;
    let name = selector::name_from_component(&canonical)?;

    let entry = component_cache_entry(paths, &name);
    if let Some(parent) = entry.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&canonical, &entry)?;

    let meta = ComponentMeta {
        source: format!("file://{}", canonical.display()),
        fetched_at: now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    let serialised = serde_saphyr::to_string(&meta)?;
    fs::write(ComponentMeta::path(paths, &name), serialised)?;
    Ok(Seeded {
        name,
        entry,
        source: canonical,
    })
}

fn ensure_component_file(path: &Path, original: &str) -> Result<(), Error> {
    if path.is_file() && path.extension().is_some_and(|ext| ext == "wasm") {
        return Ok(());
    }
    Err(Error::Diag {
        code: "adapter-component-missing",
        detail: format!(
            "adapter `{original}` did not resolve to a `.wasm` component file at {} (an \
             adapter is a single WebAssembly component)",
            path.display()
        ),
    })
}

/// Per-component provenance for a mirrored entry under
/// `<project-cache>/components/`.
///
/// The cache tenant carries its own metadata inside its own tree, one
/// sidecar per component so two seeded adapters never clobber each
/// other's provenance.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ComponentMeta {
    /// The adapter source value (a `file://` component URI) the
    /// component cache was populated from.
    pub source: String,
    /// ISO 8601 timestamp of when the component was last mirrored.
    pub fetched_at: String,
}

impl ComponentMeta {
    /// Absolute path to the `<name>.meta.yaml` provenance sidecar
    /// beside the mirrored `<name>.wasm` entry inside the out-of-tree
    /// `<project-cache>/components/` tenant.
    #[must_use]
    pub fn path(paths: &ExecutionPaths, name: &str) -> PathBuf {
        paths.cache_dir().join("components").join(format!("{name}.meta.yaml"))
    }

    /// Load the provenance sidecar for `name`, when present and
    /// parseable. The recorded `source` is the canonical `file://`
    /// URI of the component the mirror was seeded from — the value
    /// init persists on `project.yaml.adapter` for a component
    /// selector, so a guest that cannot see the operator's host path
    /// (the launcher mirrored it before the runtime started) still
    /// records the host-canonical binding.
    #[must_use]
    pub fn load(paths: &ExecutionPaths, name: &str) -> Option<Self> {
        let raw = fs::read_to_string(Self::path(paths, name)).ok()?;
        serde_saphyr::from_str(&raw).ok()
    }
}
