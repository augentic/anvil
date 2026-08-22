//! Deployment-neutral source-adapter resolution.

use emery_error::Error;
use omnia_guest::{BlobStore, StateStore};

use super::core::{
    AdapterLocation, Axis, Origin, ResolvedSource, SourceAdapter, check_requires_emery, parse_floor,
};
use super::metadata::{self, Metadata};
use super::selector::AdapterSelector;
use crate::handler::{ADAPTERS_CONTAINER, ExecutionPaths, STORE_CONTAINER};

/// Read-only component resolver over injected metadata dispatch.
pub struct Component<R: metadata::Runner> {
    metadata: R,
}

impl<R: metadata::Runner> std::fmt::Debug for Component<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Component").finish_non_exhaustive()
    }
}

impl<R: metadata::Runner> Component<R> {
    /// Creates a resolver using `metadata`.
    #[must_use]
    pub const fn new(metadata: R) -> Self {
        Self { metadata }
    }

    /// Resolves a source adapter selector.
    ///
    /// # Errors
    ///
    /// Propagates location, metadata, and compatibility failures.
    pub async fn resolve_source<S: StateStore + BlobStore>(
        &self, selector: &AdapterSelector, store: &S, paths: &ExecutionPaths,
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
        if dispatch_first(selector, &name, store, paths).await {
            let metadata = metadata::dispatch(&self.metadata, Axis::Source, &name, None)?;
            return source(&name, None, metadata, bare_origin(Axis::Source, &name));
        }
        let location = locate(Axis::Source, &name, selector.version(), store, paths).await?;
        let metadata = metadata::load(
            &self.metadata,
            store,
            &location,
            Axis::Source,
            &name,
            selector.version(),
        )
        .await?;
        source(&name, selector.version().cloned(), metadata, location.origin())
    }
}

// An unseeded bare selector delegates component location to deployment policy.
async fn dispatch_first<B: BlobStore>(
    selector: &AdapterSelector, name: &str, blobs: &B, paths: &ExecutionPaths,
) -> bool {
    matches!(selector, AdapterSelector::Bare { .. }) && !cached(blobs, name, paths).await
}

// Probe failures read as absent.
async fn cached<B: BlobStore>(blobs: &B, name: &str, paths: &ExecutionPaths) -> bool {
    let object = paths.locations().component_object(name);
    blobs.has(ADAPTERS_CONTAINER, &object).await.unwrap_or(false)
}

// Package origins describe deployment layout, not guest-visible bytes.
fn store_origin(name: &str, version: &semver::Version, paths: &ExecutionPaths) -> Origin {
    Origin {
        label: "store".to_string(),
        reference: store_reference(&paths.locations().store_object(name, &version.to_string())),
    }
}

// Dispatch-first origins carry routed identity because storage is host-owned.
fn bare_origin(axis: Axis, name: &str) -> Origin {
    Origin {
        label: "store".to_string(),
        reference: super::routed::RoutedId::new(axis, name.to_string(), None).to_string(),
    }
}

fn store_reference(object: &str) -> String {
    format!("{STORE_CONTAINER}/{object}")
}

/// Builds a resolved source and enforces its Emery CLI floor.
///
/// # Errors
///
/// Returns metadata or compatibility errors.
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

/// Locates an adapter component without dispatching metadata.
///
/// Pinned entries are verified on read; unpinned entries use the project cache.
///
/// # Errors
///
/// Returns typed not-found or verify-on-read errors.
pub async fn locate<S: StateStore + BlobStore>(
    axis: Axis, name: &str, version: Option<&semver::Version>, store: &S, paths: &ExecutionPaths,
) -> Result<AdapterLocation, Error> {
    if let Some(version) = version {
        let version = version.to_string();
        let object = paths.locations().store_object(name, &version);
        if !store.has(STORE_CONTAINER, &object).await.unwrap_or(false) {
            return Err(Error::Diag {
                code: "adapter-not-found",
                detail: format!(
                    "adapter `{name}@{version}` (axis `{axis}`) is not installed in the global \
                     store at {}; seed a local component with `emery init \
                     <path/to/{name}.wasm>` (the explicit install verb arrives with the \
                     distribution surface)",
                    store_reference(&object),
                ),
            });
        }
        verify_store_entry(axis, name, &version, store, paths).await?;
        return Ok(AdapterLocation::Store(object));
    }

    // Persisted components resolve through their mirror, surviving source removal.
    let object = paths.locations().component_object(name);
    if store.has(ADAPTERS_CONTAINER, &object).await.unwrap_or(false) {
        return Ok(AdapterLocation::Cache(object));
    }
    Err(Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "adapter `{name}` (axis `{axis}`) is not in the project component cache at \
             {ADAPTERS_CONTAINER}/{object}; seed it with `emery init <path/to/{name}.wasm>` or \
             pin a published version (`emery:{name}@<semver>`)",
        ),
    })
}

// Store entries fail closed unless bytes match their digest sidecar.
async fn verify_store_entry<S: StateStore + BlobStore>(
    axis: Axis, name: &str, version: &str, store: &S, paths: &ExecutionPaths,
) -> Result<(), Error> {
    let locations = paths.locations();
    let entry = store_reference(&locations.store_object(name, version));
    let sidecar =
        StateStore::get(store, &locations.store_meta_key(name, version)).await.ok().flatten();
    let recorded = sidecar.and_then(|bytes| {
        emery_diagnostics::cache::recorded_digest(&String::from_utf8_lossy(&bytes))
    });
    let Some(recorded) = recorded else {
        return Err(Error::Diag {
            code: "adapter-sidecar-missing",
            detail: format!(
                "store entry {entry} has no digest sidecar; unverifiable components are refused \
                 — remove the entry and install `emery:{name}@{version}` again",
            ),
        });
    };
    let unreadable = |detail: String| Error::Diag {
        code: "adapter-store-unreadable",
        detail: format!(
            "adapter `{name}@{version}` (axis `{axis}`) store entry at {entry} cannot be read \
             for verification: {detail}",
        ),
    };
    let bytes = match BlobStore::get(store, STORE_CONTAINER, &locations.store_object(name, version))
        .await
    {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Err(unreadable("the entry vanished during verification".to_string())),
        Err(err) => return Err(unreadable(format!("{err:#}"))),
    };
    let actual = emery_diagnostics::cache::content_digest(&bytes);
    if actual == recorded {
        Ok(())
    } else {
        Err(digest_mismatch(
            &format!("adapter `{name}@{version}` (axis `{axis}`) store entry at {entry}"),
            "verify-on-read",
            &emery_diagnostics::cache::DigestMismatch { recorded, actual },
        ))
    }
}

/// Builds the stable `adapter-digest-mismatch` error.
///
/// Shared construction keeps verification wording stable.
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
