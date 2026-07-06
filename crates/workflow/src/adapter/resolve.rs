//! Per-axis adapter resolver entry points.
//!
//! [`SourceAdapter::resolve`] / [`TargetAdapter::resolve`] locate the
//! single `.wasm` component for an [`AdapterRef`] identity (RFC-64),
//! obtain the cached `describe` answer ([`super::describe`]), and run
//! the post-resolve floor gate in [`super::core`].

use std::path::{Path, PathBuf};

use specify_error::Error;

use super::core::{
    AdapterLocation, AdapterRef, Axis, ResolvedSourceAdapter, ResolvedTargetAdapter, SourceAdapter,
    TargetAdapter, check_requires_specify, parse_floor,
};
use super::describe;

impl SourceAdapter {
    /// Resolve a source adapter by its [`AdapterRef`] identity
    /// (`(name, version)`).
    ///
    /// A pinned identity resolves the single-file store entry at
    /// `<store-root>/<name>@<version>.wasm` (verify-on-read included);
    /// a bare name resolves the development release build at
    /// `target/wasm32-wasip2/release/specify_<name>.wasm` under the
    /// project or the sibling `specify-adapters` checkout. Metadata
    /// comes from the component's cached `describe` answer.
    ///
    /// # Errors
    ///
    /// - `adapter-not-found` — no store entry / development artifact.
    /// - `adapter-digest-mismatch` — the store entry failed
    ///   verify-on-read.
    /// - `adapter-describe-unavailable` / `adapter-describe-failed` /
    ///   `adapter-axis-mismatch` — the describe dispatch failed.
    /// - `adapter-floor-malformed` — the describe answer's
    ///   `specify-floor` is not exact semver.
    /// - `adapter-cli-too-old` — the running binary is older than the
    ///   adapter's declared floor (RFC-47 D3, exit 3).
    pub fn resolve(
        adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedSourceAdapter, Error> {
        let name = adapter_ref.name.as_str();
        let location = locate(Axis::Source, adapter_ref, project_dir)?;
        let answer = describe::describe(&location, Axis::Source, name)?;
        let floor = parse_floor(answer.specify_floor.as_deref(), name, location.path())?;
        check_requires_specify(floor.as_ref(), env!("CARGO_PKG_VERSION"), name, location.path())?;
        Ok(ResolvedSourceAdapter {
            manifest: Self {
                name: name.to_string(),
                version: adapter_ref.resolved_version(),
                requires_specify: floor,
            },
            location,
        })
    }
}

impl TargetAdapter {
    /// Resolve a target adapter by its [`AdapterRef`] identity
    /// (`(name, version)`).
    ///
    /// Same probe and describe pipeline as [`SourceAdapter::resolve`],
    /// additionally carrying the target's declared build inputs and
    /// platforms capability from the `describe` answer.
    ///
    /// # Errors
    ///
    /// Same families as [`SourceAdapter::resolve`].
    pub fn resolve(
        adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedTargetAdapter, Error> {
        let name = adapter_ref.name.as_str();
        let location = locate(Axis::Target, adapter_ref, project_dir)?;
        let answer = describe::describe(&location, Axis::Target, name)?;
        let floor = parse_floor(answer.specify_floor.as_deref(), name, location.path())?;
        check_requires_specify(floor.as_ref(), env!("CARGO_PKG_VERSION"), name, location.path())?;
        Ok(ResolvedTargetAdapter {
            manifest: Self {
                name: name.to_string(),
                version: adapter_ref.resolved_version(),
                requires_specify: floor,
                inputs: answer.inputs,
                platforms: answer.platforms,
            },
            location,
        })
    }
}

/// The project component cache directory —
/// `<project-cache>/components/`.
///
/// The project-local probe leg for bare-name identities: `specify init`
/// mirrors an operator-supplied local `.wasm` component here so later
/// resolution stays project-local without re-reading the original path.
#[must_use]
pub fn component_cache_dir(project_dir: &Path) -> PathBuf {
    specify_schema::cache::project_cache_dir(project_dir).join("components")
}

/// Absolute path to the project component cache entry for `name` —
/// `<project-cache>/components/<name>.wasm`.
#[must_use]
pub fn component_cache_entry(project_dir: &Path, name: &str) -> PathBuf {
    component_cache_dir(project_dir).join(format!("{name}.wasm"))
}

/// The development release-build candidates for a bare-name identity.
///
/// `target/wasm32-wasip2/release/specify_<name>.wasm` under the project
/// itself, then under the sibling `specify-adapters` checkout. Built by
/// `cargo make build-guests-release` in the owning repo.
#[must_use]
pub fn dev_component_paths(project_dir: &Path, name: &str) -> Vec<PathBuf> {
    let file = dev_component_filename(name);
    let release = Path::new("target").join("wasm32-wasip2").join("release");
    let mut candidates = vec![project_dir.join(&release).join(&file)];
    if let Some(parent) = project_dir.parent() {
        candidates.push(parent.join("specify-adapters").join(&release).join(&file));
    }
    candidates
}

/// The cargo artifact filename for an adapter guest crate named
/// `specify-<name>` — `specify_<name>.wasm` with kebab dashes folded to
/// underscores.
#[must_use]
pub fn dev_component_filename(name: &str) -> String {
    format!("specify_{}.wasm", name.replace('-', "_"))
}

/// Locate the single component file for an identity.
///
/// A pinned `(name, version)` resolves only the global store entry —
/// the immutable install target the wasm-pkg transport populates —
/// with RFC-48 D4 verify-on-read against the recorded file digest. A
/// bare name resolves only the development release-build candidates.
fn locate(
    axis: Axis, adapter_ref: &AdapterRef, project_dir: &Path,
) -> Result<AdapterLocation, Error> {
    let name = adapter_ref.name.as_str();
    if let Some(version) = adapter_ref.version.as_ref() {
        let version = version.to_string();
        let entry = specify_schema::cache::adapter_store_entry(name, &version);
        if !entry.is_file() {
            return Err(Error::Diag {
                code: "adapter-not-found",
                detail: format!(
                    "adapter `{name}@{version}` (axis `{axis}`) is not installed in the global \
                     store at {}; `specify init augentic:{name}@{version}` installs the published \
                     component",
                    entry.display(),
                ),
            });
        }
        // RFC-48 D4 verify-on-read: the store entry's recorded file
        // digest must still match its current bytes, else the immutable
        // artifact has drifted (a moved tag, a corrupted store entry).
        // A missing sidecar fails open. Dev artifacts are verify-exempt
        // — only the content-addressed store is gated.
        if let Err(mismatch) = specify_schema::cache::verify_store_entry(name, &version) {
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

    // Bare-name probe order: the project component cache (a local
    // component mirrored at init) wins over the live development
    // release builds, so an explicit init-time choice stays pinned.
    let mut candidates = vec![component_cache_entry(project_dir, name)];
    candidates.extend(dev_component_paths(project_dir, name));
    if let Some(hit) = candidates.iter().find(|path| path.is_file()) {
        return Ok(AdapterLocation::Dev(hit.clone()));
    }
    let probed =
        candidates.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(", ");
    Err(Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "adapter `{name}` (axis `{axis}`) has no development artifact at {probed}; build it \
             with `cargo make build-guests-release` or pin a published version \
             (`augentic:{name}@<semver>`)",
        ),
    })
}
