//! Deployment-neutral adapter resolution and component implementation.

use std::future::Future;
use std::path::{Path, PathBuf};

use error::Error;

use super::core::{
    AdapterLocation, Axis, Origin, ResolvedSource, ResolvedTarget, SourceAdapter, TargetAdapter,
    check_requires_specify, dev_version, parse_floor,
};
use super::metadata::{self, Metadata};
use super::selector::AdapterSelector;
use crate::handler::ExecutionPaths;

/// Provider capability for resolving source and target adapters.
///
/// `resolve_*` is read-only re-resolution of an already-provisioned
/// selector. `ensure_*` owns deployment policy for making a selector
/// usable — the component deployment's package hydration, digest
/// sidecar, and local-component mirror; a native host's static catalog
/// match — before resolving it. The defaults make `ensure_*` a
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
    /// Deployment provisioning failures (hydration, digest, mirror,
    /// catalog mismatch) ahead of the resolve failures.
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
    /// Deployment provisioning failures (hydration, digest, mirror,
    /// catalog mismatch) ahead of the resolve failures.
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
        let location = locate(Axis::Source, selector, &name, paths)?;
        let metadata = metadata::load(self.metadata, &location, Axis::Source, &name)?;
        source(&name, resolved_version(selector), metadata, location.origin())
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        let name = selector.name()?;
        let location = locate(Axis::Target, selector, &name, paths)?;
        let metadata = metadata::load(self.metadata, &location, Axis::Target, &name)?;
        target(&name, resolved_version(selector), metadata, location.origin())
    }
}

/// The version a component selector resolves as: the exact package pin
/// when present, else the `0.0.0` development placeholder.
fn resolved_version(selector: &AdapterSelector) -> semver::Version {
    selector.version().cloned().unwrap_or_else(dev_version)
}

/// Build a resolved source from provider metadata, enforcing its CLI floor.
///
/// # Errors
///
/// Returns metadata, version-floor, or resolution errors.
pub fn source(
    name: &str, version: semver::Version, metadata: Metadata, origin: Origin,
) -> Result<ResolvedSource, Error> {
    let Metadata { specify_floor, .. } = metadata;
    let floor = parse_floor(specify_floor.as_deref(), name, &origin)?;
    check_requires_specify(floor.as_ref(), env!("CARGO_PKG_VERSION"), name, &origin)?;
    Ok(ResolvedSource {
        manifest: SourceAdapter {
            name: name.to_string(),
            version,
            requires_specify: floor,
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
    name: &str, version: semver::Version, metadata: Metadata, origin: Origin,
) -> Result<ResolvedTarget, Error> {
    let floor = parse_floor(metadata.specify_floor.as_deref(), name, &origin)?;
    check_requires_specify(floor.as_ref(), env!("CARGO_PKG_VERSION"), name, &origin)?;
    Ok(ResolvedTarget {
        manifest: TargetAdapter {
            name: name.to_string(),
            version,
            requires_specify: floor,
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

/// The in-repo development release-build candidate for a bare-name
/// identity. Resolution is project-contained: there is no sibling
/// checkout probe — an adapter built elsewhere reaches the project as
/// an explicitly supplied component path at init (mirrored into the
/// project component cache) or a pinned store install.
#[must_use]
pub(crate) fn dev_component_path(project_dir: &Path, name: &str) -> PathBuf {
    project_dir
        .join("target")
        .join("wasm32-wasip2")
        .join("release")
        .join(dev_component_filename(name))
}

/// Cargo artifact filename for an adapter guest crate.
#[must_use]
pub(crate) fn dev_component_filename(name: &str) -> String {
    format!("{}.wasm", name.replace('-', "_"))
}

fn locate(
    axis: Axis, selector: &AdapterSelector, name: &str, paths: &ExecutionPaths,
) -> Result<AdapterLocation, Error> {
    if let AdapterSelector::Package { version, .. } = selector {
        let version = version.to_string();
        let entry = diagnostics::cache::adapter_store_entry(name, &version);
        if !entry.is_file() {
            return Err(Error::Diag {
                code: "adapter-not-found",
                detail: format!(
                    "adapter `{name}@{version}` (axis `{axis}`) is not installed in the global \
                     store at {}; `specify init specify:{name}@{version}` installs the published \
                     component",
                    entry.display(),
                ),
            });
        }
        if let Err(mismatch) = diagnostics::cache::verify_store_entry(name, &version) {
            return Err(Error::Diag {
                code: "adapter-digest-mismatch",
                detail: format!(
                    "adapter `{name}@{version}` (axis `{axis}`) store entry at {} failed \
                     verify-on-read: recorded digest {} but recomputed {}",
                    entry.display(),
                    mismatch.recorded,
                    mismatch.actual,
                ),
            });
        }
        return Ok(AdapterLocation::Store(entry));
    }

    // Bare development shorthand and persisted local-component
    // selectors share the project-contained probe set: the mirrored
    // project component cache, then the in-repo release build. A
    // component selector resolves through its cache mirror, so it
    // keeps working after the operator's original file is removed.
    let candidates =
        [component_cache_entry(paths, name), dev_component_path(paths.project_root(), name)];
    if let Some(hit) = candidates.iter().find(|path| path.is_file()) {
        return Ok(AdapterLocation::Dev(hit.clone()));
    }
    let probed =
        candidates.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(", ");
    Err(Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "adapter `{name}` (axis `{axis}`) has no development artifact at {probed}; supply a \
             local `.wasm` component at init, build the in-repo release artifact (`cargo build \
             --release --target wasm32-wasip2`), or pin a published version \
             (`specify:{name}@<semver>`)",
        ),
    })
}
