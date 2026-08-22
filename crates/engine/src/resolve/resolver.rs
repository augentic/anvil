//! Deployment-neutral source-adapter resolution and the component
//! resolver the routed operations call directly.

use std::path::PathBuf;

use emery_error::Error;
use omnia_guest::BlobStore;

use super::core::{
    AdapterLocation, Axis, Origin, ResolvedSource, SourceAdapter, check_requires_emery, parse_floor,
};
use super::metadata::{self, Metadata};
use super::selector::AdapterSelector;
use crate::handler::{ADAPTERS_CONTAINER, ExecutionPaths, STORE_CONTAINER};

/// Component-backed resolver: read-only re-resolution of an
/// already-provisioned selector over an injected metadata dispatch
/// ([`super::ensure::source`] owns the provisioning leg).
pub struct Component<R: metadata::Runner> {
    metadata: R,
}

impl<R: metadata::Runner> std::fmt::Debug for Component<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Component").finish_non_exhaustive()
    }
}

impl<R: metadata::Runner> Component<R> {
    /// Bind component resolution to the deployment's metadata dispatch.
    #[must_use]
    pub const fn new(metadata: R) -> Self {
        Self { metadata }
    }

    /// Resolve one source adapter selector.
    ///
    /// # Errors
    ///
    /// Preserves location, metadata, and compatibility failures.
    pub async fn resolve_source<B: BlobStore>(
        &self, selector: &AdapterSelector, blobs: &B, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        let name = selector.name()?;
        if let AdapterSelector::Package { version, .. } = selector {
            let metadata = metadata::dispatch(&self.metadata, Axis::Source, &name, Some(version))?;
            return source(
                &name,
                Some(version.clone()),
                metadata,
                store_origin(&name, version, paths),
            );
        }
        if dispatch_first(selector, &name, blobs, paths).await {
            let metadata = metadata::dispatch(&self.metadata, Axis::Source, &name, None)?;
            return source(&name, None, metadata, bare_origin(Axis::Source, &name));
        }
        let location = locate(Axis::Source, &name, selector.version(), blobs, paths).await?;
        let metadata = metadata::load(
            &self.metadata,
            blobs,
            &location,
            Axis::Source,
            &name,
            selector.version(),
        )
        .await?;
        source(&name, selector.version().cloned(), metadata, location.origin())
    }
}

// Whether a bare selector must resolve dispatch-first: no seeded
// project-cache entry exists, so deployment policy locates the
// component. Resolved bare versions stay `None`.
async fn dispatch_first<B: BlobStore>(
    selector: &AdapterSelector, name: &str, blobs: &B, paths: &ExecutionPaths,
) -> bool {
    matches!(selector, AdapterSelector::Bare { .. }) && !cached(blobs, name, paths).await
}

// Whether the project component cache carries a seeded entry for
// `name`; a probe failure reads as absent, like the pre-seam
// `is_file` probe.
async fn cached<B: BlobStore>(blobs: &B, name: &str, paths: &ExecutionPaths) -> bool {
    let object = paths.locations().component_object(name);
    blobs.has(ADAPTERS_CONTAINER, &object).await.unwrap_or(false)
}

// The store identity a package pin maps to, built from the carried
// layout rather than a probed file: the origin names where the
// deployment keeps the pin, not a file the caller read.
fn store_origin(name: &str, version: &semver::Version, paths: &ExecutionPaths) -> Origin {
    Origin {
        label: "store".to_string(),
        reference: paths.locations().store_entry(name, &version.to_string()).display().to_string(),
    }
}

// The origin of a bare dispatch-first resolve: the caller never sees
// a component file (the store is host-owned with no guest mount), so
// the origin carries the routed identity, not a path.
fn bare_origin(axis: Axis, name: &str) -> Origin {
    Origin {
        label: "store".to_string(),
        reference: super::routed::RoutedId::new(axis, name.to_string(), None).to_string(),
    }
}

/// Build a resolved source from provider metadata, enforcing its CLI floor.
///
/// # Errors
///
/// Returns metadata, version-floor, or resolution errors.
pub fn source(
    name: &str, version: Option<semver::Version>, metadata: Metadata, origin: Origin,
) -> Result<ResolvedSource, Error> {
    let Metadata { emery_floor } = metadata;
    let floor = parse_floor(emery_floor.as_deref(), name, &origin)?;
    check_requires_emery(floor.as_ref(), env!("CARGO_PKG_VERSION"), name, &origin)?;
    Ok(ResolvedSource {
        manifest: SourceAdapter {
            name: name.to_string(),
            version,
            requires_emery: floor,
        },
        origin,
    })
}

/// Project component cache entry for `name`.
#[must_use]
pub(crate) fn component_cache_entry(paths: &ExecutionPaths, name: &str) -> PathBuf {
    paths.locations().component(name)
}

/// Locate the single component file for one adapter identity without
/// dispatching metadata.
///
/// Probes the verified global store entry for a version pin, else the
/// project component cache. Resolution is project-contained — no
/// sibling-checkout or build-tree probe.
///
/// # Errors
///
/// `adapter-not-found` when no probe hits; `adapter-sidecar-missing` /
/// `adapter-digest-mismatch` / `adapter-store-unreadable` when a store
/// entry fails verify-on-read.
pub async fn locate<B: BlobStore>(
    axis: Axis, name: &str, version: Option<&semver::Version>, blobs: &B, paths: &ExecutionPaths,
) -> Result<AdapterLocation, Error> {
    if let Some(version) = version {
        let version = version.to_string();
        let locations = paths.locations();
        let entry = locations.store_entry(name, &version);
        let object = locations.store_object(name, &version);
        if !blobs.has(STORE_CONTAINER, &object).await.unwrap_or(false) {
            return Err(Error::Diag {
                code: "adapter-not-found",
                detail: format!(
                    "adapter `{name}@{version}` (axis `{axis}`) is not installed in the global \
                     store at {}; seed a local component with `emery init \
                     <path/to/{name}.wasm>` (the explicit install verb arrives with the \
                     distribution surface)",
                    entry.display(),
                ),
            });
        }
        verify_store_entry(axis, name, &version, blobs, paths).await?;
        return Ok(AdapterLocation::Store(entry));
    }

    // Bare shorthand and persisted local components share the seeded
    // project-cache probe; a local component resolves through its
    // mirror, so it survives removal of the operator's original file.
    let entry = component_cache_entry(paths, name);
    if blobs
        .has(ADAPTERS_CONTAINER, &paths.locations().component_object(name))
        .await
        .unwrap_or(false)
    {
        return Ok(AdapterLocation::Cache(entry));
    }
    Err(Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "adapter `{name}` (axis `{axis}`) is not in the project component cache at {}; seed \
             it with `emery init <path/to/{name}.wasm>` or pin a published version \
             (`emery:{name}@<semver>`)",
            entry.display(),
        ),
    })
}

// Verify-on-read over the storage capability: fetch the sidecar and
// the entry bytes, then compare digests in memory (the digest math
// stays in `emery_diagnostics`). Fail-closed on every arm.
async fn verify_store_entry<B: BlobStore>(
    axis: Axis, name: &str, version: &str, blobs: &B, paths: &ExecutionPaths,
) -> Result<(), Error> {
    let locations = paths.locations();
    let entry = locations.store_entry(name, version);
    let sidecar = blobs
        .get(STORE_CONTAINER, &locations.store_meta_object(name, version))
        .await
        .ok()
        .flatten();
    let recorded = sidecar.and_then(|bytes| {
        emery_diagnostics::cache::recorded_digest(&String::from_utf8_lossy(&bytes))
    });
    let Some(recorded) = recorded else {
        return Err(Error::Diag {
            code: "adapter-sidecar-missing",
            detail: format!(
                "store entry {} has no digest sidecar; unverifiable components are refused — \
                 remove the entry and install `emery:{name}@{version}` again",
                entry.display(),
            ),
        });
    };
    let unreadable = |detail: String| Error::Diag {
        code: "adapter-store-unreadable",
        detail: format!(
            "adapter `{name}@{version}` (axis `{axis}`) store entry at {} cannot be read for \
             verification: {detail}",
            entry.display(),
        ),
    };
    let bytes = match blobs.get(STORE_CONTAINER, &locations.store_object(name, version)).await {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Err(unreadable("the entry vanished during verification".to_string())),
        Err(err) => return Err(unreadable(format!("{err:#}"))),
    };
    let actual = emery_diagnostics::cache::content_digest(&bytes);
    if actual == recorded {
        Ok(())
    } else {
        Err(digest_mismatch(
            &format!(
                "adapter `{name}@{version}` (axis `{axis}`) store entry at {}",
                entry.display()
            ),
            "verify-on-read",
            &emery_diagnostics::cache::DigestMismatch { recorded, actual },
        ))
    }
}

/// The locked `adapter-digest-mismatch` envelope for a store entry
/// whose recomputed content digest no longer matches its sidecar.
///
/// `subject` names what the caller was resolving; `phase` is the
/// verification leg (`verify-on-read` / `verify-after-write`). One
/// constructor keeps the wording identical across every verification
/// caller.
#[must_use]
pub fn digest_mismatch(
    subject: &str, phase: &str, mismatch: &emery_diagnostics::cache::DigestMismatch,
) -> Error {
    Error::Diag {
        code: "adapter-digest-mismatch",
        detail: format!(
            "{subject} failed {phase}: recorded digest {} but recomputed {}",
            mismatch.recorded, mismatch.actual,
        ),
    }
}
