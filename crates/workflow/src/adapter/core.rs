//! Axis-split adapter identity model and post-resolve coherence gates.
//!
//! Post-RFC-64 an adapter is a single WebAssembly component: identity
//! lives in the wasm-pkg package reference (`augentic:<name>@<semver>`),
//! axis in the exported world (`source` xor `target`), and the
//! remaining metadata (compatibility floor, build inputs, platforms
//! capability) in the component's own deterministic `describe` answer
//! (see [`super::describe`]). There is no on-disk manifest.
//!
//! Resolution lives in [`super::resolve`]; this module owns the typed
//! identity structs, the location enum, and the post-resolve floor
//! gate ([`check_requires_specify`]).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specify_error::Error;

use crate::Platform;
use crate::adapter::operation::{SourceOperation, TargetOperation};

/// Axis discriminator for an adapter component.
///
/// Source vs target — see workflow §Adapter vocabulary. The closed enum
/// routes the resolver dispatcher (`commands::resolve_adapter`) and the
/// describe dispatch; the in-memory adapters themselves are axis-typed
/// ([`SourceAdapter`] / [`TargetAdapter`]) so internal call sites no
/// longer carry the `axis` argument forward past the resolver boundary.
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
    /// Axis segment used by deployment guest ids and prose trees —
    /// `"sources"` for source adapters, `"targets"` for target adapters.
    #[must_use]
    pub const fn dir_segment(self) -> &'static str {
        match self {
            Self::Source => "sources",
            Self::Target => "targets",
        }
    }

    /// The `specify:adapter` axis interface a component of this axis
    /// exports — the instance name the describe dispatch invokes.
    #[must_use]
    pub const fn interface(self) -> &'static str {
        match self {
            Self::Source => "specify:adapter/source@0.1.0",
            Self::Target => "specify:adapter/target@0.1.0",
        }
    }
}

/// One adapter-declared build input from the target's `describe`
/// answer.
///
/// Each entry names a path the target's `build` operation consumes,
/// relative to the build request's `inputs.root` (the slice tree). The
/// CLI assembles the request's `inputs.artifacts.additional[]` from
/// this list and raises `target-build-input-missing` when a `required`
/// path is absent. v1 keeps the declaration a flat path list — globs
/// and conditional inputs are deferred.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildInputDeclaration {
    /// Path relative to the build request's `inputs.root`.
    pub path: String,
    /// Whether `build` requires this input; a missing `required` path
    /// is a build-time abort.
    pub required: bool,
}

/// Declarative platforms capability from a target's `describe` answer.
///
/// When a target declares `platforms`, the CLI uses this to enforce
/// platform requirements at `specify init` time and to scaffold
/// defaults for greenfield workspace members.
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

/// Where an adapter component was located on disk. The carried path is
/// the single `.wasm` component file (RFC-64 — one component, no
/// manifest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterLocation {
    /// Resolved from the global content-addressed adapter store entry
    /// at `<store-root>/<name>@<version>.wasm` (RFC-48 D5, single file
    /// post-RFC-64). The store is the immutable, version-keyed install
    /// target resolved by `specify_schema::cache::adapter_store_entry`
    /// and populated by the wasm-pkg transport. Probed whenever the
    /// [`AdapterRef`] carries a pinned version.
    Store(PathBuf),
    /// Resolved from a development release build —
    /// `target/wasm32-wasip2/release/specify_<name>.wasm` under the
    /// project itself or the sibling `specify-adapters` checkout
    /// (`cargo make build-guests-release`). Probed for bare-name
    /// (unpinned) references.
    Dev(PathBuf),
}

impl AdapterLocation {
    /// Kebab-case label for JSON envelopes (`"store"` / `"dev"`).
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Store(_) => "store",
            Self::Dev(_) => "dev",
        }
    }

    /// The component file path.
    #[must_use]
    pub const fn path(&self) -> &PathBuf {
        match self {
            Self::Store(path) | Self::Dev(path) => path,
        }
    }
}

/// The identity an adapter resolves against: a kebab-case `name` plus
/// an optional pinned semver `version` (RFC-47 D2).
///
/// Resolution keys on `(name, version)`. A `Some(_)` version is an
/// exact pin resolved against the global store entry installed for
/// that identity; `version: None` is the bare-name development
/// shorthand resolved against the sibling/in-repo release build.
/// Semver range resolution is deferred to RM-21; this value type is
/// the seam those extensions widen without re-breaking the resolve
/// call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRef {
    /// Kebab-case adapter name.
    pub name: String,
    /// Optional exact semver pin; `None` selects the development
    /// artifact.
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

    /// The version this identity resolves as: the pin when present,
    /// else the [`dev_version`] placeholder a development artifact
    /// carries.
    #[must_use]
    pub fn resolved_version(&self) -> semver::Version {
        self.version.clone().unwrap_or_else(dev_version)
    }
}

/// The placeholder version a development (unpinned) adapter resolves as.
///
/// Development components carry no package identity, so `0.0.0` is the
/// honest "not a published release" marker in topology projections and
/// envelopes.
#[must_use]
pub const fn dev_version() -> semver::Version {
    semver::Version::new(0, 0, 0)
}

/// In-memory identity + metadata of a resolved source adapter.
///
/// Constructed by [`SourceAdapter::resolve`]: `name`/`version` from the
/// [`AdapterRef`] identity, `requires_specify` from the component's
/// cached `describe` answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAdapter {
    /// Kebab-case adapter name from the resolved identity.
    pub name: String,
    /// Semver adapter version: the pin for store-resolved identities,
    /// [`dev_version`] for development artifacts.
    pub version: semver::Version,
    /// Optional host-CLI compatibility floor (RFC-47 D3) from the
    /// `describe` answer's `specify-floor`. The resolver compares it
    /// against the running binary (`check_requires_specify`) and aborts
    /// with `adapter-cli-too-old` (exit 3) when the binary is older.
    pub requires_specify: Option<semver::Version>,
}

/// In-memory identity + metadata of a resolved target adapter.
///
/// Constructed by [`TargetAdapter::resolve`]: `name`/`version` from the
/// [`AdapterRef`] identity, the rest from the component's cached
/// `describe` answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAdapter {
    /// Kebab-case adapter name from the resolved identity.
    pub name: String,
    /// Semver adapter version: the pin for store-resolved identities,
    /// [`dev_version`] for development artifacts.
    pub version: semver::Version,
    /// Optional host-CLI compatibility floor (RFC-47 D3) from the
    /// `describe` answer's `specify-floor`. The resolver compares it
    /// against the running binary (`check_requires_specify`) and aborts
    /// with `adapter-cli-too-old` (exit 3) when the binary is older.
    pub requires_specify: Option<semver::Version>,
    /// Adapter-declared build inputs from the `describe` answer. Each
    /// entry is a path relative to the build request's `inputs.root`,
    /// flagged `required`; the guest build orchestrator assembles
    /// `inputs.artifacts.additional[]` from this list.
    pub inputs: Vec<BuildInputDeclaration>,
    /// Optional platforms capability from the `describe` answer. When
    /// present the target declares the closed set of [`Platform`]
    /// tokens it accepts, whether projects must declare platforms, and
    /// the default set for greenfield scaffolding.
    pub platforms: Option<PlatformsCapability>,
}

/// A resolved [`SourceAdapter`] paired with the [`AdapterLocation`] it
/// loaded from (store entry vs. development build). The component file
/// is reachable through [`AdapterLocation::path`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSourceAdapter {
    /// Identity + describe-derived metadata.
    pub manifest: SourceAdapter,
    /// Whether the component came from the global store or a
    /// development release build, and the file itself via
    /// [`AdapterLocation::path`].
    pub location: AdapterLocation,
}

/// A resolved [`TargetAdapter`] paired with the [`AdapterLocation`] it
/// loaded from (store entry vs. development build). The component file
/// is reachable through [`AdapterLocation::path`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTargetAdapter {
    /// Identity + describe-derived metadata.
    pub manifest: TargetAdapter,
    /// Whether the component came from the global store or a
    /// development release build, and the file itself via
    /// [`AdapterLocation::path`].
    pub location: AdapterLocation,
}

impl SourceAdapter {
    /// Iterator over the source operations this adapter serves, in
    /// ascending kebab-name order (`extract < survey`) — the axis's
    /// closed WIT operation set (`wit/specify.wit`).
    pub fn operations(&self) -> impl Iterator<Item = &SourceOperation> {
        const WIT_OPERATIONS: &[SourceOperation] =
            &[SourceOperation::Extract, SourceOperation::Survey];
        WIT_OPERATIONS.iter()
    }
}

impl TargetAdapter {
    /// Iterator over the target operations this adapter serves, in
    /// ascending kebab-name order (`build < merge < shape`) — the
    /// axis's closed WIT operation set (`wit/specify.wit`:
    /// guidance/build/merge, with `shape` the historical spelling of
    /// `guidance`).
    pub fn operations(&self) -> impl Iterator<Item = &TargetOperation> {
        const WIT_OPERATIONS: &[TargetOperation] =
            &[TargetOperation::Build, TargetOperation::Merge, TargetOperation::Shape];
        WIT_OPERATIONS.iter()
    }
}

/// Parse a `describe` answer's `specify-floor` string into a typed
/// semver, naming the identity and component path on failure.
///
/// # Errors
///
/// Returns [`Error::Validation`] with the kebab discriminant
/// `adapter-floor-malformed` when the floor is not exact semver.
pub(super) fn parse_floor(
    floor: Option<&str>, name: &str, component: &std::path::Path,
) -> Result<Option<semver::Version>, Error> {
    let Some(floor) = floor else {
        return Ok(None);
    };
    semver::Version::parse(floor).map(Some).map_err(|err| {
        Error::validation_failed(
            "adapter-floor-malformed",
            "an adapter's describe answer declares a semver `specify-floor`",
            format!(
                "adapter `{name}` ({}) declares `specify-floor: {floor}`, which is not an exact semver: {err}",
                component.display(),
            ),
        )
    })
}

/// Enforce an adapter's host-CLI compatibility floor (RFC-47 D3).
///
/// `floor` is the adapter's optional `specify` minimum from its
/// `describe` answer (already parsed into a typed `semver::Version`);
/// `current` is the running binary's version (the resolve call sites
/// pass `env!("CARGO_PKG_VERSION")`, the same source [`crate::config`]
/// uses). When the binary is older than the floor the adapter cannot be
/// honored, so resolution aborts with [`Error::AdapterCliTooOld`] on
/// the exit-3 `EXIT_VERSION_TOO_OLD` path — the adapter-granularity
/// analog of the `project.yaml` `specify` floor.
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
    floor: Option<&semver::Version>, current: &str, name: &str, component: &std::path::Path,
) -> Result<(), Error> {
    let Some(floor) = floor else {
        return Ok(());
    };
    let Ok(current_version) = semver::Version::parse(current) else {
        return Ok(());
    };
    if current_version < *floor {
        return Err(Error::AdapterCliTooOld {
            adapter: format!("{name} ({})", component.display()),
            required: floor.to_string(),
            found: current.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
