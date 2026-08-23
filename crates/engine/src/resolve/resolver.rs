//! Deployment-neutral source-adapter resolution.

use emery_error::Error;
use omnia_guest::BlobStore;

use super::core::{Axis, Origin, ResolvedSource, SourceAdapter, check_requires_emery, parse_floor};
use super::metadata::{self, Metadata};
use super::selector::AdapterSelector;
use crate::handler::{ADAPTERS_CONTAINER, ExecutionPaths};
use crate::storage;

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
                routed_origin(Axis::Source, &name, Some(version)),
            );
        }
        let object = paths.locations().component_object(&name);
        let component = blobs
            .get(ADAPTERS_CONTAINER, &object)
            .await
            .map_err(|err| storage::failed("reading the component cache", &err))?;
        if matches!(selector, AdapterSelector::Bare { .. }) && component.is_none() {
            let metadata = metadata::dispatch(&self.metadata, Axis::Source, &name, None)?;
            return source(&name, None, metadata, routed_origin(Axis::Source, &name, None));
        }
        let Some(component) = component else {
            return Err(not_found(Axis::Source, &name, &object));
        };
        let metadata =
            metadata::load(&self.metadata, blobs, &object, &component, Axis::Source, &name).await?;
        source(&name, None, metadata, cache_origin(&object))
    }
}

// Routed origins name the host-selected guest; no stored bytes are implied.
fn routed_origin(axis: Axis, name: &str, version: Option<&semver::Version>) -> Origin {
    Origin {
        label: "route".to_string(),
        reference: super::routed::RoutedId::new(axis, name.to_string(), version.cloned())
            .to_string(),
    }
}

fn cache_origin(object: &str) -> Origin {
    Origin {
        label: "cache".to_string(),
        reference: format!("{ADAPTERS_CONTAINER}/{object}"),
    }
}

fn not_found(axis: Axis, name: &str, object: &str) -> Error {
    Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "adapter `{name}` (axis `{axis}`) is not in the project component cache at \
             {ADAPTERS_CONTAINER}/{object}; seed it with `emery specify <path/to/{name}.wasm>` \
             or use a statically admitted package pin (`emery:{name}@<semver>`)",
        ),
    }
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
