//! Axis-split adapter manifest model and post-load coherence gates.
//!
//! Source adapters and target adapters share a manifest shape on the
//! wire (`adapter.yaml`) but carry disjoint closed operation sets:
//! [`SourceOperation`] (`extract | survey`) vs. [`TargetOperation`]
//! (`shape | build | merge`). The in-memory split into
//! [`SourceAdapter`] / [`TargetAdapter`] pushes the kebab-string
//! boundary out to the YAML parse step — `briefs.keys()` is the typed
//! operation iterator for full-shape manifests (with serde rejecting
//! any unknown variant before downstream code ever sees a string), and
//! briefless post-cutover manifests fall back to the axis's closed WIT
//! operation set (`wit/specify.wit`).
//!
//! Resolution lives in [`super::resolve`]; schema validation and the
//! cross-axis collision probe in [`super::validate_manifest`]. This
//! module owns the manifest types, the path helpers, and the post-load
//! coherence gates ([`check_axis_and_name`] and friends).
//!
//! See [DECISIONS.md §"Operations typed at parse boundary"] for the
//! rationale.
//!
//! [DECISIONS.md §"Operations typed at parse boundary"]: ../../../../DECISIONS.md#operations-typed-at-parse-boundary

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specify_error::Error;
use specify_extension::ExtensionPermissions;

use crate::Platform;
use crate::adapter::operation::{SourceOperation, TargetOperation};

/// Filename of an adapter manifest.
///
/// Source and target adapters share the `adapter.yaml` filename per
/// workflow §Adapter implementation shape; the directory's axis (under
/// `adapters/sources/` or `adapters/targets/`) and the manifest's
/// `axis:` field disambiguate.
pub const ADAPTER_FILENAME: &str = "adapter.yaml";

/// Committed WASI extension binary at an adapter's root (RFC-48 D3/D10).
///
/// When an adapter declares an `extension`, its WASI component is the
/// committed `adapter.wasm` beside `adapter.yaml`; `specify extension
/// run <name>` resolves the binary from the installed adapter tree
/// rather than a `tools.yaml` sidecar (RFC-48 D11).
pub const ADAPTER_WASM_FILENAME: &str = "adapter.wasm";

/// Parent directory for in-repo adapter trees.
pub const ADAPTERS_DIR: &str = "adapters";

/// Manifest-cache tenant segment under the out-of-tree project cache.
///
/// `<project-cache>/manifests/{sources,targets}/<name>/` mirrors the
/// in-repo `adapters/{sources,targets}/<name>/` tree (see
/// [DECISIONS.md §"Cache layout"]).
///
/// [DECISIONS.md §"Cache layout"]: ../../../DECISIONS.md#cache-layout
pub const MANIFESTS_CACHE_DIR: &str = "manifests";

/// Axis discriminator for an adapter manifest.
///
/// Source vs target — see workflow §Adapter vocabulary. The closed enum is
/// used by the resolver dispatcher (`commands::resolve_adapter`) and
/// the manifest-cache helpers ([`cache_dir`], `adapter_axis_dir`);
/// the in-memory manifests themselves are axis-typed
/// ([`SourceAdapter`] / [`TargetAdapter`]) so internal call sites no
/// longer carry the `axis` argument forward past the resolver
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Axis {
    /// Source adapter — `extract` + `survey`.
    Source,
    /// Target adapter — `shape` + `build` + `merge`.
    Target,
}

impl Axis {
    /// Axis segment under `ADAPTERS_DIR` — `"sources"` for source
    /// adapters, `"targets"` for target adapters.
    #[must_use]
    pub const fn dir_segment(self) -> &'static str {
        match self {
            Self::Source => "sources",
            Self::Target => "targets",
        }
    }

    /// The complementary axis. Used by the cross-axis uniqueness
    /// probe (see [DECISIONS.md §"Adapter name uniqueness"]) to
    /// reject a name that resolves under both `adapters/sources/` and
    /// `adapters/targets/`.
    ///
    /// [DECISIONS.md §"Adapter name uniqueness"]: ../../../../DECISIONS.md#adapter-name-uniqueness
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Source => Self::Target,
            Self::Target => Self::Source,
        }
    }
}

/// `<project_dir>/adapters/{sources,targets}/` for the given axis.
#[must_use]
pub fn adapter_axis_dir(project_dir: &Path, axis: Axis) -> PathBuf {
    project_dir.join(ADAPTERS_DIR).join(axis.dir_segment())
}

/// The singular WASI extension declared inside an adapter manifest
/// (RFC-48 D11).
///
/// An adapter ships at most one binary, so the manifest carries one
/// `extension` object rather than a `tools[]` array. Decoupled from
/// [`specify_extension::Extension`] so adapter loading does not
/// pull in the WASI runtime surface; the wasm builds from the co-located
/// `extension/` crate (D10) and rides the adapter's own semver identity
/// (RFC-47), so a per-extension `version` / `source` / `sha256` is
/// rejected by the schema. `permissions` reuses
/// [`ExtensionPermissions`] — the `{read, write}` shape the WASI runner
/// already speaks.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdapterExtensionDeclaration {
    /// Optional run handle for `specify extension run <name>`; defaults
    /// to the adapter name when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Structured WASI filesystem permissions in the `{read, write}`
    /// shape.
    #[serde(default, skip_serializing_if = "ExtensionPermissions::is_default")]
    pub permissions: ExtensionPermissions,
}

/// Optional prepare hook declared on a target adapter manifest.
///
/// When present, `specify slice build --phase prepare` dispatches
/// `extension run <name> <argv...> <slice-dir>` before the build brief
/// handoff. Requires a declared `extension` (enforced by schema and the
/// loader post-schema gate).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PrepareHookDeclaration {
    /// Extension subcommand prefix; the host appends the slice directory.
    pub argv: Vec<String>,
}

/// Optional native build hook declared on a target adapter manifest.
///
/// When present, `specify slice build` executes the script on the host
/// with `SPECIFY_PROJECT_DIR` and `SPECIFY_SLICE_DIR` set. `host_prereq`
/// runs at prepare (before the manifest `prepare` extension hook);
/// `finalize_verify` runs at finalize on a clean `status: success` report
/// before the `built` transition.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NativeBuildHookDeclaration {
    /// Path relative to the adapter root.
    pub script: String,
}

/// Optional catalog capabilities declared on a target adapter manifest.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub struct CatalogCapability {
    /// When true, `specify catalog infer` may dispatch the extension's
    /// `infer` subcommand.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub infer: bool,
}

/// One adapter-declared build input inside a target manifest.
///
/// Each entry names a path the target's `build` operation consumes,
/// relative to the build request's `inputs.root` (the slice tree). The
/// CLI assembles the request's `inputs.artifacts.additional[]` from
/// this list and (in a later change) raises `target-build-input-missing`
/// when a `required` path is absent. v1 keeps the declaration a flat
/// path list — globs and conditional inputs are deferred.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildInputDeclaration {
    /// Path relative to the build request's `inputs.root`.
    pub path: String,
    /// Whether `build` requires this input; a missing `required` path
    /// is a build-time abort once the matching check lands.
    pub required: bool,
}

/// Declarative platforms capability for a target adapter manifest.
///
/// When a target declares `platforms` in its `adapter.yaml`, the CLI
/// uses this to enforce platform requirements at `specify init` time
/// and to scaffold defaults for greenfield workspace members.
///
/// - `required` — if true, `specify init` demands `--platforms`.
/// - `allowed` — the closed set of [`Platform`] tokens the target
///   accepts; any project token outside the set is rejected.
/// - `default` — the platform set scaffolded when the operator does
///   not specify (used by greenfield workspace sync).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlatformsCapability {
    /// Whether projects using this target must declare platforms.
    pub required: bool,
    /// Platforms this target accepts.
    pub allowed: Vec<Platform>,
    /// Default platform set for greenfield scaffolding.
    pub default: Vec<Platform>,
}

/// Typed outcome of [`PlatformsCapability::check`].
///
/// Each caller maps the violation onto its own diagnostic-code family
/// (`project-platforms-*` at init, `topology-cache-project-platforms-*`
/// at topology resolution) so the rules themselves live in one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformsViolation {
    /// The capability demands a platform set but none was declared.
    /// Carries the capability's display-formatted `default` set for the
    /// caller's hint text.
    RequiredButMissing {
        /// Display-formatted `default` platform tokens.
        defaults: Vec<String>,
    },
    /// A non-empty platform set omits the mandatory `core` member.
    MissingCore,
    /// A declared platform is outside the capability's `allowed` set.
    /// Carries the display-formatted allowed set for the hint text.
    NotAllowed {
        /// The offending platform.
        platform: Platform,
        /// Display-formatted `allowed` platform tokens.
        allowed: Vec<String>,
    },
}

impl PlatformsCapability {
    /// Validate a declared platform set against this capability: a
    /// required capability refuses an empty set; a non-empty set must
    /// include [`Platform::Core`] and stay inside `allowed`. An empty
    /// set on a non-required capability passes (platforms are opt-in).
    ///
    /// # Errors
    ///
    /// Returns the first [`PlatformsViolation`] in rule order.
    pub fn check(&self, platforms: &[Platform]) -> Result<(), PlatformsViolation> {
        if platforms.is_empty() {
            if self.required {
                return Err(PlatformsViolation::RequiredButMissing {
                    defaults: self.default.iter().map(ToString::to_string).collect(),
                });
            }
            return Ok(());
        }
        if !platforms.contains(&Platform::Core) {
            return Err(PlatformsViolation::MissingCore);
        }
        for p in platforms {
            if !self.allowed.contains(p) {
                return Err(PlatformsViolation::NotAllowed {
                    platform: *p,
                    allowed: self.allowed.iter().map(ToString::to_string).collect(),
                });
            }
        }
        Ok(())
    }
}

/// Closed adapter execution mode.
///
/// Declared by the required `execution:` field on `adapter.yaml`.
/// Source adapters are agent-only (`source.schema.json` enumerates
/// `["agent"]`); target adapters may still declare `tool`, though the
/// target-side `build` / `merge` dispatch carries `agent` as a
/// placeholder. See DECISIONS.md §"Adapter execution mode".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Execution {
    /// `execution: agent` — the adapter's brief is executed by an agent
    /// against the sandbox preopens. The CLI orchestrates inputs and
    /// validates outputs against the schemas; agent outputs are
    /// non-deterministic, so nothing is memoized.
    Agent,
    /// `execution: tool` — target-axis only: `build` / `merge` are
    /// dispatched through a declared WASI extension or a built-in
    /// deterministic Rust path. Source adapters are agent-only.
    Tool,
}

/// Where an adapter manifest was located on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterLocation {
    /// Resolved from the global content-addressed adapter store entry at
    /// `<store-root>/<name>@<version>/` (RFC-48 D5). The store is the
    /// immutable, version-keyed install target resolved by
    /// `specify_schema::cache::adapter_store_entry` and populated by the
    /// registry transport. Probed first whenever the [`AdapterRef`]
    /// carries a pinned version.
    Store(PathBuf),
    /// Resolved from `<project_dir>/adapters/{sources,targets}/<name>/`.
    Local(PathBuf),
    /// Resolved from the out-of-tree manifest cache at
    /// `<project-cache>/manifests/{sources,targets}/<name>/`.
    /// The manifest cache mirrors the in-repo adapter tree
    /// (`adapter.yaml` plus brief markdown) — see
    /// [DECISIONS.md §"Cache layout"].
    ///
    /// [DECISIONS.md §"Cache layout"]: ../../../../DECISIONS.md#cache-layout
    Cached(PathBuf),
}

impl AdapterLocation {
    /// Kebab-case label for JSON envelopes (`"store"` / `"local"` /
    /// `"cached"`).
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Store(_) => "store",
            Self::Local(_) => "local",
            Self::Cached(_) => "cached",
        }
    }

    /// Underlying filesystem path.
    #[must_use]
    pub const fn path(&self) -> &PathBuf {
        match self {
            Self::Store(path) | Self::Local(path) | Self::Cached(path) => path,
        }
    }
}

/// Manifest cache root for an axis —
/// `<project-cache>/manifests/{sources,targets}/`, resolved out-of-tree
/// from the OS cache (see [`crate::config::Layout::cache_dir`]).
///
/// Path-only helper — the directory may or may not exist on disk.
#[must_use]
pub fn cache_axis_dir(project_dir: &Path, axis: Axis) -> PathBuf {
    crate::config::Layout::new(project_dir)
        .cache_dir()
        .join(MANIFESTS_CACHE_DIR)
        .join(axis.dir_segment())
}

/// Manifest cache root for `(axis, name)` —
/// `<project-cache>/manifests/{sources,targets}/<name>/`.
///
/// This is the agent-populated mirror of `adapters/{sources,targets}/<name>/`
/// — `adapter.yaml` plus the brief markdown files it references. See
/// [DECISIONS.md §"Cache layout"].
///
/// Path-only helper — the directory may or may not exist on disk.
///
/// [DECISIONS.md §"Cache layout"]: ../../../../DECISIONS.md#cache-layout
#[must_use]
pub fn cache_dir(project_dir: &Path, axis: Axis, name: &str) -> PathBuf {
    cache_axis_dir(project_dir, axis).join(name)
}

/// Per-operation agent scratch lane for `(adapter, segment)` —
/// `.specify/scratch/<adapter>/<segment>/`.
///
/// `<segment>` is the literal `survey` for the slice-less survey op or
/// the slice name for extract.
/// The write-only `$SCRATCH_DIR` preopen of the source-operation
/// sandbox. Rooted under the transient in-tree working-state tree
/// (`.specify/scratch/`), structurally disjoint from the out-of-tree
/// memoization cache, so a scratch write can never pollute a cache
/// artifact; see [DECISIONS.md §"Cache layout"].
///
/// Path-only helper — the directory may or may not exist on disk.
///
/// [DECISIONS.md §"Cache layout"]: ../../../../DECISIONS.md#cache-layout
#[must_use]
pub fn scratch_dir(project_dir: &Path, adapter: &str, segment: &str) -> PathBuf {
    crate::config::Layout::new(project_dir).scratch_dir().join(adapter).join(segment)
}

/// The identity an adapter resolves against: a kebab-case `name` plus
/// an optional pinned semver `version` (RFC-47 D2).
///
/// Resolution keys on `(name, version)`. `version: None` is the
/// bare-name shorthand — it resolves the single installed identity for
/// `name`, or raises `adapter-version-required` when a single identity
/// cannot be picked. A `Some(_)` version is an exact pin: resolution
/// matches it against the installed manifest by equality, and a pin
/// that names no installed identity is `adapter-version-required` as
/// well ("resolution cannot pick a single installed identity for the
/// name"). Semver range resolution is deferred to RM-21; this value
/// type is the seam those extensions widen without re-breaking the
/// resolve call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRef {
    /// Kebab-case adapter name.
    pub name: String,
    /// Optional exact semver pin; `None` selects the single installed
    /// identity.
    pub version: Option<semver::Version>,
}

impl AdapterRef {
    /// A bare-name reference with no version pin.
    #[must_use]
    pub fn bare(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
        }
    }

    /// A reference pinned to an exact semver version.
    #[must_use]
    pub fn pinned(name: impl Into<String>, version: semver::Version) -> Self {
        Self {
            name: name.into(),
            version: Some(version),
        }
    }
}

/// In-memory representation of a source-adapter manifest
/// (`adapters/sources/<name>/adapter.yaml`).
///
/// Constructed by [`SourceAdapter::resolve`] after the wire YAML has
/// been validated against `schemas/adapter.schema.json` +
/// `schemas/source.schema.json`. The typed `briefs` map carries the
/// closed [`SourceOperation`] set — unknown keys are rejected at
/// serde-parse time before this struct is ever materialised.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceAdapter {
    /// Kebab-case adapter name; must match the directory under
    /// `adapters/sources/<name>/`.
    pub name: String,
    /// Semver adapter version. Resolution keys on this as identity
    /// (see [`super::AdapterRef`]); the per-axis JSON Schema enforces
    /// the semver `pattern` and `check_version` is the typed
    /// belt-and-suspenders gate (`adapter-version-malformed`).
    pub version: semver::Version,
    /// Optional host-CLI compatibility floor (RFC-47 D3): the exact
    /// minimum `specify` platform version this adapter needs,
    /// deserialized from the `specify` manifest key. The loader compares
    /// it against the running binary at resolve time
    /// (`check_requires_specify`) and aborts with `adapter-cli-too-old`
    /// (exit 3) when the binary is older. `None` means no floor — the
    /// field is optional and back-compatible.
    #[serde(default, rename = "specify", skip_serializing_if = "Option::is_none")]
    pub requires_specify: Option<semver::Version>,
    /// Axis discriminator on the wire. Always [`Axis::Source`] after a
    /// successful [`SourceAdapter::resolve`]; the field is retained
    /// so YAML round-trips byte-for-byte through serde.
    pub axis: Axis,
    /// Closed adapter execution mode. Optional on the wire (RFC-61
    /// two-shape window): shrunk post-cutover manifests omit it, and
    /// native consumers that dispatch on it fail at the point of use
    /// (`adapter-execution-mode-required`) rather than at resolve time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<Execution>,
    /// Typed source-operation → relative brief path map. Optional on
    /// the wire (RFC-61 two-shape window): when present,
    /// `source.schema.json#/properties/briefs` closes it to the full
    /// `extract` + `survey` set; when absent (empty map), the operation
    /// set derives from the closed WIT contract instead — see
    /// [`SourceAdapter::operations`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub briefs: BTreeMap<SourceOperation, String>,
    /// Optional singular declared WASI extension (RFC-48 D11).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<AdapterExtensionDeclaration>,
    /// Optional human-readable summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// In-memory representation of a target-adapter manifest
/// (`adapters/targets/<name>/adapter.yaml`).
///
/// Constructed by [`TargetAdapter::resolve`] after the wire YAML has
/// been validated against `schemas/adapter.schema.json` +
/// `schemas/target.schema.json`. The typed `briefs` map carries the
/// closed [`TargetOperation`] set — unknown keys are rejected at
/// serde-parse time before this struct is ever materialised.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TargetAdapter {
    /// Kebab-case adapter name; must match the directory under
    /// `adapters/targets/<name>/`.
    pub name: String,
    /// Semver adapter version. Resolution keys on this as identity
    /// (see [`super::AdapterRef`]); the per-axis JSON Schema enforces
    /// the semver `pattern` and `check_version` is the typed
    /// belt-and-suspenders gate (`adapter-version-malformed`).
    pub version: semver::Version,
    /// Optional host-CLI compatibility floor (RFC-47 D3): the exact
    /// minimum `specify` platform version this adapter needs,
    /// deserialized from the `specify` manifest key. The loader compares
    /// it against the running binary at resolve time
    /// (`check_requires_specify`) and aborts with `adapter-cli-too-old`
    /// (exit 3) when the binary is older. `None` means no floor — the
    /// field is optional and back-compatible.
    #[serde(default, rename = "specify", skip_serializing_if = "Option::is_none")]
    pub requires_specify: Option<semver::Version>,
    /// Axis discriminator on the wire. Always [`Axis::Target`] after
    /// a successful [`TargetAdapter::resolve`]; the field is retained
    /// so YAML round-trips byte-for-byte through serde.
    pub axis: Axis,
    /// Closed adapter execution mode. Optional on the wire (RFC-61
    /// two-shape window): shrunk post-cutover manifests omit it, and
    /// native consumers that dispatch on it fail at the point of use
    /// (`adapter-execution-mode-required`) rather than at resolve time.
    /// Full-shape first-party target manifests carry `agent` as a
    /// placeholder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<Execution>,
    /// Typed target-operation → relative brief path map. Optional on
    /// the wire (RFC-61 two-shape window): when present,
    /// `target.schema.json#/properties/briefs` closes it to the full
    /// `shape` + `build` + `merge` set; when absent (empty map), the
    /// operation set derives from the closed WIT contract instead —
    /// see [`TargetAdapter::operations`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub briefs: BTreeMap<TargetOperation, String>,
    /// Optional singular declared WASI extension (RFC-48 D11).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<AdapterExtensionDeclaration>,
    /// Optional adapter-declared build inputs. Each entry is
    /// a path relative to the build request's `inputs.root`, flagged
    /// `required`; the CLI assembles `inputs.artifacts.additional[]`
    /// from this list. Defaults to an empty list when omitted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<BuildInputDeclaration>,
    /// Optional human-readable summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional platforms capability. When present the target declares
    /// the closed set of [`Platform`] tokens it accepts, whether
    /// projects must declare platforms, and the default set for
    /// greenfield scaffolding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<PlatformsCapability>,
    /// Optional prepare hook for `specify slice build --phase prepare`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepare: Option<PrepareHookDeclaration>,
    /// Optional host toolchain gate at `specify slice build --phase prepare`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_prereq: Option<NativeBuildHookDeclaration>,
    /// Optional host verify backstop at `specify slice build --phase finalize`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finalize_verify: Option<NativeBuildHookDeclaration>,
    /// Optional catalog capabilities (`specify catalog infer`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<CatalogCapability>,
}

/// A parsed [`SourceAdapter`] paired with the [`AdapterLocation`] it
/// loaded from (in-repo vs. agent-populated cache). The filesystem
/// directory is reachable through [`AdapterLocation::path`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSourceAdapter {
    /// Parsed manifest.
    pub manifest: SourceAdapter,
    /// Whether the manifest came from the out-of-tree
    /// `<project-cache>/manifests/sources/<name>/` or from
    /// `<project_dir>/adapters/sources/<name>/`, and the directory
    /// itself via [`AdapterLocation::path`].
    pub location: AdapterLocation,
}

/// A parsed [`TargetAdapter`] paired with the [`AdapterLocation`] it
/// loaded from (in-repo vs. agent-populated cache). The filesystem
/// directory is reachable through [`AdapterLocation::path`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTargetAdapter {
    /// Parsed manifest.
    pub manifest: TargetAdapter,
    /// Whether the manifest came from the out-of-tree
    /// `<project-cache>/manifests/targets/<name>/` or from
    /// `<project_dir>/adapters/targets/<name>/`, and the directory
    /// itself via [`AdapterLocation::path`].
    pub location: AdapterLocation,
}

impl SourceAdapter {
    /// Iterator over the source operations this adapter declares, in
    /// ascending kebab-name order (`extract < survey`). For full-shape
    /// manifests `briefs.keys()` is the canonical typed operation
    /// source; a briefless post-cutover manifest falls back to the
    /// axis's closed WIT operation set (`wit/specify.wit`), which the
    /// full `briefs` key set equals by schema.
    pub fn operations(&self) -> impl Iterator<Item = &SourceOperation> {
        const WIT_OPERATIONS: &[SourceOperation] =
            &[SourceOperation::Extract, SourceOperation::Survey];
        let fallback: &[SourceOperation] =
            if self.briefs.is_empty() { WIT_OPERATIONS } else { &[] };
        self.briefs.keys().chain(fallback)
    }
}

impl TargetAdapter {
    /// Iterator over the target operations this adapter declares, in
    /// ascending kebab-name order (`build < merge < shape`). For
    /// full-shape manifests `briefs.keys()` is the canonical typed
    /// operation source; a briefless post-cutover manifest falls back
    /// to the axis's closed WIT operation set (`wit/specify.wit`:
    /// guidance/build/merge, with `shape` the manifest spelling of
    /// `guidance`), which the full `briefs` key set equals by schema.
    pub fn operations(&self) -> impl Iterator<Item = &TargetOperation> {
        const WIT_OPERATIONS: &[TargetOperation] =
            &[TargetOperation::Build, TargetOperation::Merge, TargetOperation::Shape];
        let fallback: &[TargetOperation] =
            if self.briefs.is_empty() { WIT_OPERATIONS } else { &[] };
        self.briefs.keys().chain(fallback)
    }
}

/// Resolve the WASI extension run handle for a target adapter.
///
/// Returns `None` when the manifest declares no `extension`. When the
/// declaration omits `extension.name`, the adapter `name` is the
/// default run handle — mirroring [`AdapterExtensionDeclaration`]'s
/// contract in the extension inventory assembler.
#[must_use]
pub fn extension_run_name(adapter: &TargetAdapter) -> Option<String> {
    adapter.extension.as_ref().map(|ext| ext.name.clone().unwrap_or_else(|| adapter.name.clone()))
}

/// Post-load gate: `prepare` requires a declared `extension`.
///
/// The target-axis JSON Schema also enforces this via `if/then`; this
/// typed gate is belt-and-suspenders for manifests that bypass schema
/// validation.
pub(super) fn check_prepare_requires_extension(
    manifest: &TargetAdapter, manifest_path: &Path,
) -> Result<(), Error> {
    if manifest.prepare.is_some() && manifest.extension.is_none() {
        return Err(Error::validation_failed(
            "adapter-prepare-without-extension",
            "a target manifest with `prepare` must also declare `extension`",
            format!("{} declares `prepare` but omits `extension`", manifest_path.display()),
        ));
    }
    Ok(())
}

/// Post-load axis/name coherence gate, run by [`super::resolve`] after
/// schema validation against the typed manifest fields.
///
/// Returns `Error::Diag` with `adapter-axis-mismatch` when the
/// manifest's `axis:` disagrees with the resolver's axis, and
/// `adapter-name-mismatch` when `name:` disagrees with the directory
/// the manifest lives under.
pub(super) fn check_axis_and_name(
    expected_axis: Axis, expected_name: &str, manifest_axis: Axis, manifest_name: &str,
    manifest_path: &Path,
) -> Result<(), Error> {
    if manifest_axis != expected_axis {
        return Err(Error::Diag {
            code: "adapter-axis-mismatch",
            detail: format!(
                "{} declares axis `{manifest_axis}`, but resolver was asked for axis `{expected_axis}`",
                manifest_path.display(),
            ),
        });
    }
    if manifest_name != expected_name {
        return Err(Error::Diag {
            code: "adapter-name-mismatch",
            detail: format!(
                "{} declares name `{manifest_name}` but lives under `{expected_name}/`",
                manifest_path.display(),
            ),
        });
    }
    Ok(())
}

/// Typed `version` gate, run on the raw manifest value before the
/// typed deserialise so a malformed semver surfaces as the specific
/// `adapter-version-malformed` finding rather than the free-form
/// `adapter-manifest-malformed` serde error.
///
/// Mirrors `specify_extension`'s `tool.version-is-semver` rule:
/// the per-axis JSON Schemas already mark `version` with the semver
/// `pattern`, so this typed gate is the belt-and-suspenders for a
/// manifest that reaches the loader through a path that bypassed
/// schema validation.
///
/// # Errors
///
/// Returns [`Error::Validation`] with the kebab discriminant
/// `adapter-version-malformed` when `version` is absent, non-string,
/// or not parseable as an exact semver.
pub(super) fn check_version(
    raw_value: &serde_json::Value, manifest_path: &Path,
) -> Result<(), Error> {
    let raw_version = raw_value.get("version").and_then(serde_json::Value::as_str);
    let Some(raw_version) = raw_version else {
        return Err(Error::validation_failed(
            "adapter-version-malformed",
            "adapter manifest declares a semver `version` string",
            format!("{} omits a string `version` field", manifest_path.display()),
        ));
    };
    if let Err(err) = semver::Version::parse(raw_version) {
        return Err(Error::validation_failed(
            "adapter-version-malformed",
            "adapter manifest declares a semver `version` string",
            format!(
                "{} declares `version: {raw_version}`, which is not an exact semver: {err}",
                manifest_path.display(),
            ),
        ));
    }
    Ok(())
}

/// Match the requested [`AdapterRef`] version against the single
/// installed manifest identity (RFC-47 D2).
///
/// Resolution is project-local with exactly one installed identity per
/// `name`, so a `None` pin always picks that identity. A `Some(_)` pin
/// must equal the installed `version`; a pin that names a different
/// version cannot be satisfied, so resolution "cannot pick a single
/// installed identity for the name" — the `adapter-version-required`
/// case. Exact pins only; semver range matching is deferred to RM-21.
///
/// # Errors
///
/// Returns [`Error::Validation`] with the kebab discriminant
/// `adapter-version-required` when a pinned version does not match the
/// installed identity.
pub(super) fn check_requested_version(
    requested: Option<&semver::Version>, name: &str, installed: &semver::Version,
    manifest_path: &Path,
) -> Result<(), Error> {
    if let Some(requested) = requested
        && requested != installed
    {
        return Err(Error::validation_failed(
            "adapter-version-required",
            "a version pin resolves a single installed adapter identity",
            format!(
                "{} requested `{name}@{requested}`, but the installed identity is `{name}@{installed}`",
                manifest_path.display(),
            ),
        ));
    }
    Ok(())
}

/// Enforce an adapter's host-CLI compatibility floor (RFC-47 D3).
///
/// `floor` is the adapter's optional `specify` minimum (already parsed
/// into a typed `semver::Version`); `current` is the running binary's
/// version (the resolve call sites pass `env!("CARGO_PKG_VERSION")`,
/// the same source [`crate::config`] uses). When the binary is older
/// than the floor the adapter cannot be honored, so resolution aborts
/// with [`Error::AdapterCliTooOld`] on the exit-3 `EXIT_VERSION_TOO_OLD`
/// path — the adapter-granularity analog of the `project.yaml`
/// `specify` floor.
///
/// `current` is parsed permissively: an unparseable running version is
/// treated as "not older" rather than bricking resolution, mirroring
/// `config::version_is_older`. An absent `floor` is a clean pass.
///
/// # Errors
///
/// Returns [`Error::AdapterCliTooOld`] when `current` parses below
/// `floor`.
pub(super) fn check_requires_specify(
    floor: Option<&semver::Version>, current: &str, name: &str, manifest_path: &Path,
) -> Result<(), Error> {
    let Some(floor) = floor else {
        return Ok(());
    };
    let Ok(current_version) = semver::Version::parse(current) else {
        return Ok(());
    };
    if current_version < *floor {
        return Err(Error::AdapterCliTooOld {
            adapter: format!("{name} ({})", manifest_path.display()),
            required: floor.to_string(),
            found: current.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
