//! Component-deployment kernels behind the provider's ensure legs.
//!
//! Only a local component selector provisions here; bare names and
//! package pins provision nothing in-guest.

use std::fs;
use std::path::{Path, PathBuf};

use emery_error::Error;
use omnia_guest::BlobStore;
use serde::{Deserialize, Serialize};

use super::core::ResolvedSource;
use super::resolver::{Component, component_cache_entry};
use super::selector::canonicalize_component;
use super::{AdapterSelector, metadata, selector};
use crate::handler::{ADAPTERS_CONTAINER, ExecutionPaths};
use crate::storage;

/// Ensure a source selector for the component deployment: provision
/// (mirror), then resolve through the component resolver.
///
/// # Errors
///
/// Provisioning failures (`adapter-component-missing`,
/// `adapter-canonicalize-failed`) ahead of resolve failures.
pub async fn source<B: BlobStore>(
    runner: impl metadata::Runner, selector: &AdapterSelector, blobs: &B, paths: &ExecutionPaths,
    now: jiff::Timestamp,
) -> Result<ResolvedSource, Error> {
    provision(selector, blobs, paths, now).await?;
    Component::new(runner).resolve_source(selector, blobs, paths).await
}

/// Make one selector resolvable on the guest side of the seam: mirror
/// a local component into the project cache, or nothing for a bare
/// development name or a package pin (host-installed on dispatch).
///
/// # Errors
///
/// `adapter-component-missing` or `adapter-canonicalize-failed`.
pub async fn provision<B: BlobStore>(
    selector: &AdapterSelector, blobs: &B, paths: &ExecutionPaths, now: jiff::Timestamp,
) -> Result<(), Error> {
    match selector {
        AdapterSelector::Bare { .. } | AdapterSelector::Package { .. } => Ok(()),
        AdapterSelector::Component { path } => mirror(path, blobs, paths, now).await,
    }
}

// Mirror an operator-supplied local component into the project cache,
// stamping provenance: a component selector stays resolvable after the
// original file is removed because the earlier mirror satisfies re-ensure.
async fn mirror<B: BlobStore>(
    path: &Path, blobs: &B, paths: &ExecutionPaths, now: jiff::Timestamp,
) -> Result<(), Error> {
    let absolute =
        if path.is_absolute() { path.to_path_buf() } else { paths.project_root().join(path) };
    if !absolute.is_file() {
        let cached = match selector::name_from_component(path) {
            Ok(name) => {
                let object = paths.locations().component_object(&name);
                blobs.has(ADAPTERS_CONTAINER, &object).await.unwrap_or(false)
            }
            Err(_) => false,
        };
        if cached {
            return Ok(());
        }
    }
    seed(path, blobs, paths, now).await.map(drop)
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
/// Canonicalizes, derives the kebab name from the filename, mirrors
/// the bytes into the components container, and stamps provenance.
/// Re-seeding replaces the entry; a wrong-world component fails at the
/// dispatch gate, not during seeding. Strict: a missing path fails
/// even when the derived name is already cached (no typo masking).
///
/// # Errors
///
/// `adapter-component-missing` when `path` is not a `.wasm` file,
/// `adapter-canonicalize-failed` when it cannot be canonicalized, and
/// read or storage failures from the mirror or provenance write.
pub async fn seed<B: BlobStore>(
    path: &Path, blobs: &B, paths: &ExecutionPaths, now: jiff::Timestamp,
) -> Result<Seeded, Error> {
    let absolute =
        if path.is_absolute() { path.to_path_buf() } else { paths.project_root().join(path) };
    ensure_component_file(&absolute, &path.display().to_string())?;
    let canonical = canonicalize_component(path, paths.project_root())?;
    let name = selector::name_from_component(&canonical)?;
    let locations = paths.locations();

    // Reading the operator-supplied component is a workspace read;
    // the mirror itself goes through the storage capability.
    let bytes = fs::read(&canonical)?;
    blobs
        .put(ADAPTERS_CONTAINER, &locations.component_object(&name), &bytes)
        .await
        .map_err(|err| storage::failed("mirroring the component into the cache", &err))?;

    let meta = ComponentMeta {
        source: format!("file://{}", canonical.display()),
        fetched_at: now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    let serialised = serde_saphyr::to_string(&meta)?;
    blobs
        .put(ADAPTERS_CONTAINER, &locations.component_meta_object(&name), serialised.as_bytes())
        .await
        .map_err(|err| storage::failed("stamping the component provenance", &err))?;
    Ok(Seeded {
        entry: component_cache_entry(paths, &name),
        name,
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
    /// Load the provenance sidecar for `name`, when present and
    /// parseable. The recorded `source` is the canonical `file://`
    /// URI of the component the mirror was seeded from — the value
    /// init persists on the source binding for a component selector,
    /// so a guest that cannot see the operator's host path still
    /// records the host-canonical binding.
    pub async fn load<B: BlobStore>(blobs: &B, paths: &ExecutionPaths, name: &str) -> Option<Self> {
        let object = paths.locations().component_meta_object(name);
        let bytes = blobs.get(ADAPTERS_CONTAINER, &object).await.ok().flatten()?;
        serde_saphyr::from_str(&String::from_utf8_lossy(&bytes)).ok()
    }
}
