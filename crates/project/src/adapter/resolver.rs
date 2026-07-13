//! Deployment-neutral adapter resolution and component implementation.

use std::path::{Path, PathBuf};

use error::Error;

use super::core::{
    AdapterLocation, AdapterRef, Axis, Origin, ResolvedSource, ResolvedTarget, SourceAdapter,
    TargetAdapter, check_requires_specify, parse_floor,
};
use super::metadata::{self, Metadata};

/// Provider capability for resolving source and target adapters.
pub trait Resolver: Send + Sync {
    /// Resolve one source adapter identity.
    ///
    /// # Errors
    ///
    /// Preserves location, metadata, and compatibility failures.
    fn resolve_source(
        &self, adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedSource, Error>;

    /// Resolve one target adapter identity.
    ///
    /// # Errors
    ///
    /// Preserves location, metadata, and compatibility failures.
    fn resolve_target(
        &self, adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedTarget, Error>;
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
        &self, adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedSource, Error> {
        let name = adapter_ref.name.as_str();
        let location = locate(Axis::Source, adapter_ref, project_dir)?;
        let metadata = metadata::load(self.metadata, &location, Axis::Source, name)?;
        source(adapter_ref, metadata, location.origin())
    }

    fn resolve_target(
        &self, adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedTarget, Error> {
        let name = adapter_ref.name.as_str();
        let location = locate(Axis::Target, adapter_ref, project_dir)?;
        let metadata = metadata::load(self.metadata, &location, Axis::Target, name)?;
        target(adapter_ref, metadata, location.origin())
    }
}

/// Build a resolved source from provider metadata, enforcing its CLI floor.
///
/// # Errors
///
/// Returns metadata, version-floor, or resolution errors.
pub fn source(
    adapter_ref: &AdapterRef, metadata: Metadata, origin: Origin,
) -> Result<ResolvedSource, Error> {
    let name = adapter_ref.name.as_str();
    let Metadata { specify_floor, .. } = metadata;
    let floor = parse_floor(specify_floor.as_deref(), name, &origin)?;
    check_requires_specify(floor.as_ref(), env!("CARGO_PKG_VERSION"), name, &origin)?;
    Ok(ResolvedSource {
        manifest: SourceAdapter {
            name: name.to_string(),
            version: adapter_ref.resolved_version(),
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
    adapter_ref: &AdapterRef, metadata: Metadata, origin: Origin,
) -> Result<ResolvedTarget, Error> {
    let name = adapter_ref.name.as_str();
    let floor = parse_floor(metadata.specify_floor.as_deref(), name, &origin)?;
    check_requires_specify(floor.as_ref(), env!("CARGO_PKG_VERSION"), name, &origin)?;
    Ok(ResolvedTarget {
        manifest: TargetAdapter {
            name: name.to_string(),
            version: adapter_ref.resolved_version(),
            requires_specify: floor,
            inputs: metadata.inputs,
            platforms: metadata.platforms,
        },
        origin,
    })
}

/// Project component cache directory.
#[must_use]
pub(crate) fn component_cache_dir(project_dir: &Path) -> PathBuf {
    diagnostics::cache::project_cache_dir(project_dir).join("components")
}

/// Project component cache entry for `name`.
#[must_use]
pub(crate) fn component_cache_entry(project_dir: &Path, name: &str) -> PathBuf {
    component_cache_dir(project_dir).join(format!("{name}.wasm"))
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
    axis: Axis, adapter_ref: &AdapterRef, project_dir: &Path,
) -> Result<AdapterLocation, Error> {
    let name = adapter_ref.name.as_str();
    if let Some(version) = adapter_ref.version.as_ref() {
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

    let candidates =
        [component_cache_entry(project_dir, name), dev_component_path(project_dir, name)];
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
