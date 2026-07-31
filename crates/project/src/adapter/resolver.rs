//! Deployment-neutral adapter resolution and component implementation.

use std::future::Future;
use std::path::PathBuf;

use error::Error;

use super::core::{
    AdapterLocation, Axis, Origin, ResolvedSource, ResolvedTarget, SourceAdapter, TargetAdapter,
    check_requires_emery, parse_floor,
};
use super::metadata::{self, Metadata};
use super::selector::AdapterSelector;
use crate::handler::ExecutionPaths;

/// Provider capability for resolving source and target adapters.
///
/// `resolve_*` is read-only re-resolution of an already-provisioned
/// selector. `ensure_*` owns deployment policy for making a selector
/// usable — the component deployment's local-component mirror; a
/// native host's static catalog match — before resolving it. Package
/// installation is host-owned (the launcher installs a missing pin
/// during metadata dispatch), so a package pin ensures without
/// guest-side provisioning. The defaults make `ensure_*` a
/// side-effect-free resolve for deployments with nothing to provision.
pub trait Resolver: Send + Sync {
    /// Resolve one source adapter selector.
    ///
    /// # Errors
    ///
    /// Preserves location, metadata, and compatibility failures.
    fn resolve_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error>;

    /// Resolve one target adapter selector.
    ///
    /// # Errors
    ///
    /// Preserves location, metadata, and compatibility failures.
    fn resolve_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error>;

    /// Make `selector` usable under this deployment, then resolve it.
    ///
    /// # Errors
    ///
    /// Deployment provisioning failures (mirror, digest, catalog
    /// mismatch) ahead of the resolve failures.
    fn ensure_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> impl Future<Output = Result<ResolvedSource, Error>> + Send {
        let resolved = self.resolve_source(selector, paths);
        async move { resolved }
    }

    /// Make `selector` usable under this deployment, then resolve it.
    ///
    /// # Errors
    ///
    /// Deployment provisioning failures (mirror, digest, catalog
    /// mismatch) ahead of the resolve failures.
    fn ensure_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> impl Future<Output = Result<ResolvedTarget, Error>> + Send {
        let resolved = self.resolve_target(selector, paths);
        async move { resolved }
    }
}

/// Component-backed resolver used by the shipped WASI provider.
#[derive(Clone, Copy, Debug)]
pub struct Component {
    metadata: metadata::Runner,
}

impl Component {
    /// Bind component resolution to the deployment's metadata dispatch.
    #[must_use]
    pub const fn new(metadata: metadata::Runner) -> Self {
        Self { metadata }
    }
}

impl Resolver for Component {
    fn resolve_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        let name = selector.name()?;
        if let AdapterSelector::Package { version, .. } = selector {
            let metadata = metadata::dispatch(self.metadata, Axis::Source, &name, Some(version))?;
            return source(
                &name,
                Some(version.clone()),
                metadata,
                store_origin(&name, version, paths),
            );
        }
        if dispatch_first(selector, &name, paths) {
            let metadata = metadata::dispatch(self.metadata, Axis::Source, &name, None)?;
            return source(&name, None, metadata, bare_origin(Axis::Source, &name));
        }
        let location = locate(Axis::Source, selector, &name, paths)?;
        let metadata =
            metadata::load(self.metadata, &location, Axis::Source, &name, selector.version())?;
        source(&name, selector.version().cloned(), metadata, location.origin())
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        let name = selector.name()?;
        if let AdapterSelector::Package { version, .. } = selector {
            let metadata = metadata::dispatch(self.metadata, Axis::Target, &name, Some(version))?;
            return target(
                &name,
                Some(version.clone()),
                metadata,
                store_origin(&name, version, paths),
            );
        }
        if dispatch_first(selector, &name, paths) {
            let metadata = metadata::dispatch(self.metadata, Axis::Target, &name, None)?;
            return target(&name, None, metadata, bare_origin(Axis::Target, &name));
        }
        let location = locate(Axis::Target, selector, &name, paths)?;
        let metadata =
            metadata::load(self.metadata, &location, Axis::Target, &name, selector.version())?;
        target(&name, selector.version().cloned(), metadata, location.origin())
    }
}

/// Whether a bare selector must resolve dispatch-first: no seeded
/// project-cache entry exists, so local-first deployment policy on the
/// other side of the seam locates the component (the newest installed
/// store version, with a pull-latest provisioning leg when nothing
/// local exists). The guest never learns which version the deployment
/// chose — resolved bare versions stay `None`.
fn dispatch_first(selector: &AdapterSelector, name: &str, paths: &ExecutionPaths) -> bool {
    matches!(selector, AdapterSelector::Bare { .. })
        && !component_cache_entry(paths, name).is_file()
}

/// The deployment-neutral origin of a package-pin resolve: the global
/// store identity the pin maps to.
///
/// Built from the carried layout rather than a probed file — package
/// metadata dispatches by routed id before any store file is visible
/// to the caller (the host resolver installs a missing pin during that
/// dispatch), so the origin names where the deployment keeps the pin,
/// not a file the caller read.
fn store_origin(name: &str, version: &semver::Version, paths: &ExecutionPaths) -> Origin {
    Origin {
        label: "store".to_string(),
        reference: paths.locations().store_entry(name, &version.to_string()).display().to_string(),
    }
}

/// The deployment-neutral origin of a bare dispatch-first resolve: the
/// routed id the deployment resolved local-first. The caller never
/// sees a component file (the store is host-owned with no guest
/// mount), so the origin carries the identity, not a path.
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
    let Metadata { emery_floor, .. } = metadata;
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

/// Build a resolved target from provider metadata, enforcing its CLI floor.
///
/// # Errors
///
/// Returns metadata, version-floor, or resolution errors.
pub fn target(
    name: &str, version: Option<semver::Version>, metadata: Metadata, origin: Origin,
) -> Result<ResolvedTarget, Error> {
    let floor = parse_floor(metadata.emery_floor.as_deref(), name, &origin)?;
    check_requires_emery(floor.as_ref(), env!("CARGO_PKG_VERSION"), name, &origin)?;
    Ok(ResolvedTarget {
        manifest: TargetAdapter {
            name: name.to_string(),
            version,
            requires_emery: floor,
            inputs: metadata.inputs,
            platforms: metadata.platforms,
        },
        origin,
    })
}

/// Project component cache directory under the execution context's
/// cache placement.
#[must_use]
pub(crate) fn component_cache_dir(paths: &ExecutionPaths) -> PathBuf {
    paths.cache_dir().join("components")
}

/// Project component cache entry for `name`.
#[must_use]
pub(crate) fn component_cache_entry(paths: &ExecutionPaths, name: &str) -> PathBuf {
    component_cache_dir(paths).join(format!("{name}.wasm"))
}

/// Locate the single component file for one selector without
/// dispatching metadata.
///
/// Probes the verified global store entry for a package pin, else the
/// project component cache for a bare or local-component selector.
/// Resolution is project-contained: there is no sibling-checkout or
/// build-tree probe — an adapter built elsewhere reaches the project
/// through `emery adapter add` (or a local component at init) or a
/// pinned store install.
///
/// The metadata-free half of [`Component`] resolution — the deployment
/// launcher derives closure component paths through it before the
/// runtime (and any metadata dispatch) exists.
///
/// # Errors
///
/// `adapter-not-found` when no probe hits; `adapter-digest-mismatch`
/// when a store entry fails verify-on-read.
pub fn locate(
    axis: Axis, selector: &AdapterSelector, name: &str, paths: &ExecutionPaths,
) -> Result<AdapterLocation, Error> {
    if let AdapterSelector::Package { version, .. } = selector {
        let version = version.to_string();
        let entry = paths.locations().store_entry(name, &version);
        if !entry.is_file() {
            return Err(Error::Diag {
                code: "adapter-not-found",
                detail: format!(
                    "adapter `{name}@{version}` (axis `{axis}`) is not installed in the global \
                     store at {}; `emery init emery:{name}@{version}` installs the published \
                     component",
                    entry.display(),
                ),
            });
        }
        let meta = paths.locations().store_meta(name, &version);
        match diagnostics::cache::verify_store_entry(&entry, &meta) {
            Ok(()) => {}
            Err(diagnostics::cache::StoreVerifyError::MissingSidecar) => {
                return Err(Error::Diag {
                    code: "adapter-sidecar-missing",
                    detail: format!(
                        "store entry {} has no digest sidecar; unverifiable components are \
                         refused — reinstall `emery:{name}@{version}` to record one",
                        entry.display(),
                    ),
                });
            }
            Err(diagnostics::cache::StoreVerifyError::Mismatch(mismatch)) => {
                return Err(digest_mismatch(
                    &format!(
                        "adapter `{name}@{version}` (axis `{axis}`) store entry at {}",
                        entry.display()
                    ),
                    "verify-on-read",
                    &mismatch,
                ));
            }
        }
        return Ok(AdapterLocation::Store(entry));
    }

    // Bare shorthand and persisted local-component selectors share
    // the single project-contained probe: the seeded project
    // component cache. A component selector resolves through its
    // cache mirror, so it keeps working after the operator's original
    // file is removed.
    let entry = component_cache_entry(paths, name);
    if entry.is_file() {
        return Ok(AdapterLocation::Cache(entry));
    }
    Err(Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "adapter `{name}` (axis `{axis}`) is not in the project component cache at {}; seed \
             it with `emery adapter add <path/to/{name}.wasm>` or pin a published version \
             (`emery:{name}@<semver>`)",
            entry.display(),
        ),
    })
}

/// The locked `adapter-digest-mismatch` envelope for a store entry
/// whose recomputed content digest no longer matches its sidecar.
///
/// `subject` names what the caller was resolving; `phase` is the
/// verification leg (`verify-on-read` / `verify-after-write`). One
/// constructor keeps the wording identical across [`locate`] and the
/// deployment launcher's install leg.
#[must_use]
pub fn digest_mismatch(
    subject: &str, phase: &str, mismatch: &diagnostics::cache::DigestMismatch,
) -> Error {
    Error::Diag {
        code: "adapter-digest-mismatch",
        detail: format!(
            "{subject} failed {phase}: recorded digest {} but recomputed {}",
            mismatch.recorded, mismatch.actual,
        ),
    }
}
