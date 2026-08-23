//! Component provisioning and resolution.

use std::fs;
use std::path::{Path, PathBuf};

use emery_error::Error;
use omnia_guest::BlobStore;

use super::core::ResolvedSource;
use super::resolver::Component;
use super::selector::canonicalize_component;
use super::{AdapterSelector, metadata, selector};
use crate::handler::{ADAPTERS_CONTAINER, ExecutionPaths, preopen_path};
use crate::storage;

/// Provisions and resolves a source selector.
///
/// # Errors
///
/// Returns provisioning or resolution failures.
pub async fn source<B: BlobStore>(
    runner: impl metadata::Runner, selector: &AdapterSelector, blobs: &B, paths: &ExecutionPaths,
) -> Result<ResolvedSource, Error> {
    provision(selector, blobs, paths).await?;
    Component::new(runner).resolve_source(selector, blobs, paths).await
}

/// Mirrors local components; bare names and package pins require no guest provisioning.
///
/// # Errors
///
/// Returns `adapter-component-missing` or `adapter-canonicalize-failed`.
pub async fn provision<B: BlobStore>(
    selector: &AdapterSelector, blobs: &B, paths: &ExecutionPaths,
) -> Result<(), Error> {
    match selector {
        AdapterSelector::Bare { .. } | AdapterSelector::Package { .. } => Ok(()),
        AdapterSelector::Component { path } => mirror(path, blobs, paths).await,
    }
}

// An existing mirror keeps a recorded selector resolvable after source removal.
async fn mirror<B: BlobStore>(path: &Path, blobs: &B, paths: &ExecutionPaths) -> Result<(), Error> {
    let relative = preopen_path(path, "<adapter>")?;
    let absolute = paths.project_root().join(&relative);
    if !absolute.is_file() {
        let cached = match selector::name_from_component(&relative) {
            Ok(name) => {
                let object = paths.locations().component_object(&name);
                blobs
                    .has(ADAPTERS_CONTAINER, &object)
                    .await
                    .map_err(|err| storage::failed("probing the component cache", &err))?
            }
            Err(_) => false,
        };
        if cached {
            return Ok(());
        }
    }
    seed(&relative, blobs, paths).await.map(drop)
}

/// Result of seeding a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seeded {
    /// Adapter name derived from the filename.
    pub name: String,
    /// Canonical source path.
    pub source: PathBuf,
}

/// Seeds a `.wasm` component into the project cache.
///
/// Re-seeding replaces the entry. World validation remains a dispatch
/// concern, and missing paths fail even if the name is already cached.
///
/// # Errors
///
/// Returns path, canonicalization, read, or storage failures.
pub async fn seed<B: BlobStore>(
    path: &Path, blobs: &B, paths: &ExecutionPaths,
) -> Result<Seeded, Error> {
    let relative = preopen_path(path, "<adapter>")?;
    let absolute = paths.project_root().join(&relative);
    ensure_component_file(&absolute, &path.display().to_string())?;
    let canonical = canonicalize_component(&relative, paths.project_root())?;
    let name = selector::name_from_component(&canonical)?;

    // Source reads use the workspace; mirrors use the storage capability.
    let bytes = fs::read(&canonical)?;
    blobs
        .put(ADAPTERS_CONTAINER, &paths.locations().component_object(&name), &bytes)
        .await
        .map_err(|err| storage::failed("mirroring the component into the cache", &err))?;
    Ok(Seeded {
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
