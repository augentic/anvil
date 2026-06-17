//! Per-axis adapter resolver entry points.
//!
//! [`SourceAdapter::resolve`] / [`TargetAdapter::resolve`] probe the
//! manifest cache then the in-repo tree (workflow §Resolver and cache),
//! schema-validate via [`super::validate_manifest`], and run the
//! post-load coherence gates in [`super::core`].

use std::path::{Path, PathBuf};

use specify_error::Error;

use super::core::{
    ADAPTER_FILENAME, AdapterLocation, AdapterRef, Axis, ResolvedSourceAdapter,
    ResolvedTargetAdapter, SourceAdapter, TargetAdapter, adapter_axis_dir, cache_dir,
    check_axis_and_name, check_execution, check_requested_version, check_requires_specify,
    check_version,
};
use super::validate_manifest::{axis_collision_error, sibling_manifest_path, validate_schema};

impl SourceAdapter {
    /// Resolve a source adapter by its [`AdapterRef`] identity
    /// (`(name, version)`).
    ///
    /// Probe order, per workflow §Resolver and cache:
    ///
    /// 1. `<store-root>/<name>@<version>/adapter.yaml` — the global
    ///    content-addressed adapter store (RFC-48 D5), probed only when
    ///    `adapter_ref.version` carries a pin.
    /// 2. `<project-cache>/manifests/sources/<name>/adapter.yaml`
    ///    (agent-populated out-of-tree manifest cache).
    /// 3. `<project_dir>/adapters/sources/<name>/adapter.yaml` (in-repo).
    ///
    /// The probe keys on `adapter_ref.name` (and, for the store entry,
    /// the pinned version); a `Some(_)` version pin is matched against
    /// the installed manifest by equality after load (RFC-47 D2).
    ///
    /// # Errors
    ///
    /// Returns `Error::Diag` with one of the following codes:
    /// - `adapter-not-found` — neither cache nor local directory exists.
    /// - `adapter-manifest-missing` — directory exists but no `adapter.yaml`.
    /// - `adapter-manifest-read-failed` — manifest exists but cannot be read.
    /// - `adapter-manifest-malformed` — manifest parses as something
    ///   other than the [`SourceAdapter`] shape.
    /// - `adapter-axis-mismatch` — manifest's `axis:` does not match
    ///   [`Axis::Source`].
    /// - `adapter-name-mismatch` — manifest's `name:` does not match
    ///   `adapter_ref.name`.
    /// - `adapter-schema-violation` — manifest fails the source-axis
    ///   JSON Schema.
    /// - `adapter-version-malformed` — manifest `version` is not semver.
    /// - `adapter-version-required` — a version pin does not match the
    ///   installed identity.
    /// - `adapter-cli-too-old` — the running binary is older than the
    ///   adapter's declared `specify` floor (RFC-47 D3, exit 3).
    pub fn resolve(
        adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedSourceAdapter, Error> {
        let name = adapter_ref.name.as_str();
        let (manifest, location, manifest_path) =
            resolve_typed::<Self>(Axis::Source, adapter_ref, project_dir)?;
        check_axis_and_name(Axis::Source, name, manifest.axis, &manifest.name, &manifest_path)?;
        check_execution(manifest.execution, &manifest_path)?;
        check_requested_version(
            adapter_ref.version.as_ref(),
            name,
            &manifest.version,
            &manifest_path,
        )?;
        check_requires_specify(
            manifest.requires_specify.as_ref(),
            env!("CARGO_PKG_VERSION"),
            name,
            &manifest_path,
        )?;
        Ok(ResolvedSourceAdapter { manifest, location })
    }
}

impl TargetAdapter {
    /// Resolve a target adapter by its [`AdapterRef`] identity
    /// (`(name, version)`).
    ///
    /// Probe order, per workflow §Resolver and cache:
    ///
    /// 1. `<store-root>/<name>@<version>/adapter.yaml` — the global
    ///    content-addressed adapter store (RFC-48 D5), probed only when
    ///    `adapter_ref.version` carries a pin.
    /// 2. `<project-cache>/manifests/targets/<name>/adapter.yaml`
    ///    (agent-populated out-of-tree manifest cache).
    /// 3. `<project_dir>/adapters/targets/<name>/adapter.yaml` (in-repo).
    ///
    /// The probe keys on `adapter_ref.name` (and, for the store entry,
    /// the pinned version); a `Some(_)` version pin is matched against
    /// the installed manifest by equality after load (RFC-47 D2).
    ///
    /// # Errors
    ///
    /// Returns `Error::Diag` with one of the following codes:
    /// - `adapter-not-found` — neither cache nor local directory exists.
    /// - `adapter-manifest-missing` — directory exists but no `adapter.yaml`.
    /// - `adapter-manifest-read-failed` — manifest exists but cannot be read.
    /// - `adapter-manifest-malformed` — manifest parses as something
    ///   other than the [`TargetAdapter`] shape.
    /// - `adapter-axis-mismatch` — manifest's `axis:` does not match
    ///   [`Axis::Target`].
    /// - `adapter-name-mismatch` — manifest's `name:` does not match
    ///   `adapter_ref.name`.
    /// - `adapter-schema-violation` — manifest fails the target-axis
    ///   JSON Schema.
    /// - `adapter-version-malformed` — manifest `version` is not semver.
    /// - `adapter-version-required` — a version pin does not match the
    ///   installed identity.
    /// - `adapter-cli-too-old` — the running binary is older than the
    ///   adapter's declared `specify` floor (RFC-47 D3, exit 3).
    pub fn resolve(
        adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedTargetAdapter, Error> {
        let name = adapter_ref.name.as_str();
        let (manifest, location, manifest_path) =
            resolve_typed::<Self>(Axis::Target, adapter_ref, project_dir)?;
        check_axis_and_name(Axis::Target, name, manifest.axis, &manifest.name, &manifest_path)?;
        check_execution(manifest.execution, &manifest_path)?;
        check_requested_version(
            adapter_ref.version.as_ref(),
            name,
            &manifest.version,
            &manifest_path,
        )?;
        check_requires_specify(
            manifest.requires_specify.as_ref(),
            env!("CARGO_PKG_VERSION"),
            name,
            &manifest_path,
        )?;
        Ok(ResolvedTargetAdapter { manifest, location })
    }
}

/// Locate, schema-validate, and typed-deserialise an adapter manifest
/// of the axis-specific shape `M`.
///
/// Wraps [`load_validated`] with the `serde_json::from_value` step that
/// was duplicated byte-for-byte between [`SourceAdapter::resolve`] and
/// [`TargetAdapter::resolve`]. Returns the parsed manifest, its
/// [`AdapterLocation`], and the canonical manifest path; callers run the
/// axis/name coherence check ([`check_axis_and_name`]) against the
/// typed manifest fields and wrap the result into the matching
/// `Resolved*Adapter`.
fn resolve_typed<M: serde::de::DeserializeOwned>(
    axis: Axis, adapter_ref: &AdapterRef, project_dir: &Path,
) -> Result<(M, AdapterLocation, PathBuf), Error> {
    let (location, manifest_path, raw_value) = load_validated(axis, adapter_ref, project_dir)?;
    let manifest: M = serde_json::from_value(raw_value).map_err(|err| Error::Diag {
        code: "adapter-manifest-malformed",
        detail: format!("failed to deserialize {}: {err}", manifest_path.display()),
    })?;
    Ok((manifest, location, manifest_path))
}

/// Shared load + schema-validate pipeline used by both axis-specific
/// resolvers.
///
/// Returns the [`AdapterLocation`] tag (whose [`AdapterLocation::path`]
/// is the root directory), the canonical manifest path (for error
/// messages), and the schema-validated `serde_json::Value` ready for
/// typed deserialisation by [`resolve_typed`].
fn load_validated(
    axis: Axis, adapter_ref: &AdapterRef, project_dir: &Path,
) -> Result<(AdapterLocation, PathBuf, serde_json::Value), Error> {
    let location = locate_axis(axis, adapter_ref, project_dir)?;
    let root_dir = location.path();
    let manifest_path = root_dir.join(ADAPTER_FILENAME);
    if !manifest_path.is_file() {
        return Err(Error::Diag {
            code: "adapter-manifest-missing",
            detail: format!("no `adapter.yaml` at {}", root_dir.display()),
        });
    }
    let raw = std::fs::read_to_string(&manifest_path).map_err(|err| Error::Diag {
        code: "adapter-manifest-read-failed",
        detail: format!("failed to read adapter manifest {}: {err}", manifest_path.display()),
    })?;

    // Validate against the schema first so a more specific error
    // bubbles up than serde's free-form parse failure.
    let raw_value: serde_json::Value = serde_saphyr::from_str(&raw).map_err(|err| Error::Diag {
        code: "adapter-manifest-malformed",
        detail: format!("failed to parse {}: {err}", manifest_path.display()),
    })?;
    validate_schema(axis, &manifest_path, &raw_value)?;
    check_version(&raw_value, &manifest_path)?;

    Ok((location, manifest_path, raw_value))
}

fn locate_axis(
    axis: Axis, adapter_ref: &AdapterRef, project_dir: &Path,
) -> Result<AdapterLocation, Error> {
    let name = adapter_ref.name.as_str();
    // RFC-48 D5: a pinned `(name, version)` resolves first against the
    // global content-addressed adapter store, the immutable install
    // target the registry transport populates. The store is keyed by
    // version, so it is only probed when the ref carries one.
    let store_version = adapter_ref.version.as_ref().map(ToString::to_string);
    let store = store_version
        .as_ref()
        .map(|version| specify_schema::cache::adapter_store_entry(name, version));
    let cached = cache_dir(project_dir, axis, name);
    let local = adapter_axis_dir(project_dir, axis).join(name);
    // Probe order: global store entry (pinned only) → out-of-tree
    // manifest cache → in-repo `adapters/{sources,targets}/<name>/`.
    // The manifest cache owns its own out-of-tree root
    // (`<project-cache>/manifests/{sources,targets}/<name>/`) — see
    // [DECISIONS.md §"Cache layout"].
    let location = if let Some(entry) = store.as_ref().filter(|entry| entry.is_dir()) {
        // RFC-48 D4 verify-on-read: a store entry's recorded tree digest
        // must still match its current content, else the immutable
        // artifact has drifted (a moved tag, a corrupted store entry). A
        // missing sidecar fails open. `Cached` / `Local` are
        // verify-exempt — only the content-addressed store is gated.
        if let Some(version) = store_version.as_deref()
            && let Err(mismatch) = specify_schema::cache::verify_store_entry(name, version)
        {
            return Err(Error::Diag {
                code: "adapter-digest-mismatch",
                detail: format!(
                    "adapter `{name}` (axis `{axis}`) store entry at {} failed verify-on-read: recorded tree digest {} but recomputed {}",
                    entry.display(),
                    mismatch.recorded,
                    mismatch.actual,
                ),
            });
        }
        AdapterLocation::Store(entry.clone())
    } else if cached.is_dir() {
        AdapterLocation::Cached(cached)
    } else if local.is_dir() {
        AdapterLocation::Local(local)
    } else {
        let store_hint =
            store.as_ref().map_or_else(String::new, |entry| format!("{}, ", entry.display()));
        return Err(Error::Diag {
            code: "adapter-not-found",
            detail: format!(
                "adapter `{name}` (axis `{axis}`) not found at {store_hint}{} or {}",
                cached.display(),
                local.display(),
            ),
        });
    };
    // Cross-axis uniqueness invariant — see DECISIONS.md
    // §"Adapter name uniqueness". `specify` is fork-and-exit, so the
    // pair of `is_file` probes below is cheaper than memoising them
    // behind process-global state; `init` / `init --workspace` and the
    // manifest-cache write boundary call [`super::check_axis_unique_for_name`]
    // eagerly on the same invariant.
    if let Some(sibling) = sibling_manifest_path(axis.opposite(), name, project_dir) {
        return Err(axis_collision_error(name, axis, location.path(), &sibling));
    }
    Ok(location)
}
