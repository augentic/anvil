//! Axis-split adapter identity model and post-resolve coherence gates.
//!
//! An adapter is a single WebAssembly component: identity
//! lives in the wasm-pkg package reference (`specify:<name>@<semver>`),
//! axis in the exported world (`source` xor `target`), and the
//! remaining metadata (compatibility floor, build inputs, platforms
//! in the component's own deterministic `metadata` export
//! (see [`super::metadata`]). There is no on-disk manifest.
//!
//! Resolution lives in [`super::resolve`]; this module owns the typed
//! identity structs, the location enum, and the post-resolve floor
//! gate ([`check_requires_specify`]).

use std::path::PathBuf;

use error::Error;
use serde::{Deserialize, Serialize};

use crate::Platform;
use crate::adapter::operation::{SourceOperation, TargetOperation};

/// Axis discriminator for an adapter component.
///
/// Source vs target — see workflow §Adapter vocabulary. The closed enum
/// routes the resolver dispatcher (`commands::resolve_adapter`) and the
/// metadata dispatch; the in-memory adapters themselves are axis-typed
/// ([`SourceAdapter`] / [`TargetAdapter`]) so internal call sites no
/// longer carry the `axis` argument forward past the resolver boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Axis {
    /// Source adapter — `extract` + `survey`.
    Source,
    /// Target adapter — `guidance` + `build` + `merge`.
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

    /// Axis prefix of a routed adapter id — the `<axis>` in
    /// `<axis>:<name>`, the id the engine names on every seam call.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Target => "target",
        }
    }
}

/// One adapter-declared build input from the target's `metadata`
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
    /// Slice-relative input path.
    pub path: String,
    /// Whether `build` requires this input; a missing `required` path
    /// is a build-time abort.
    pub required: bool,
}

/// Declarative platforms capability from a target's metadata answer.
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
/// Each caller surface owns a diagnostic-code family
/// (`project-platforms-*` at init, `topology-cache-project-platforms-*`
/// at topology resolution); the shared `PlatformsViolation::into_error`
/// converter keeps both mappings — and the rules — in one place.
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
        platform: Platform,
        /// Display-formatted `allowed` platform tokens.
        allowed: Vec<String>,
    },
}

/// Which caller surface a [`PlatformsViolation`] is being reported
/// from. Selects the diagnostic-code family and message wording in
/// `PlatformsViolation::into_error`.
#[derive(Debug, Clone, Copy)]
pub enum PlatformsSurface<'a> {
    /// `specify init --platforms` validation — the
    /// `project-platforms-*` family.
    Init {
        /// Target adapter name for the message text.
        target: &'a str,
    },
    /// Workspace topology backstop validation — the
    /// `topology-cache-project-platforms-*` family.
    TopologySlot {
        /// Workspace slot (registry) name for the message text.
        registry: &'a str,
        /// Target adapter name for the message text.
        target: &'a str,
    },
}

impl PlatformsViolation {
    /// Convert this violation into the engine [`Error`] for the given
    /// caller surface, preserving each surface's locked diagnostic
    /// codes and message wording.
    pub(crate) fn into_error(self, surface: PlatformsSurface<'_>) -> Error {
        match (self, surface) {
            (Self::RequiredButMissing { defaults }, PlatformsSurface::Init { target }) => {
                Error::validation_failed(
                    "project-platforms-required",
                    format!("target '{target}' requires --platforms"),
                    format!(
                        "target '{target}' requires --platforms; default set is [{}]",
                        defaults.join(", "),
                    ),
                )
            }
            (
                Self::RequiredButMissing { defaults },
                PlatformsSurface::TopologySlot { registry, target },
            ) => Error::validation_failed(
                "topology-cache-project-platforms-missing",
                format!("workspace slot `{registry}` declares platforms"),
                format!(
                    "workspace slot `{registry}` target '{target}' requires platforms but \
                     project.yaml declares none; default set is [{}]",
                    defaults.join(", "),
                ),
            ),
            (Self::MissingCore, PlatformsSurface::Init { .. }) => Error::validation_failed(
                "project-platforms-must-include-core",
                "platform set must include `core`",
                "the --platforms set must include `core`; every project that declares platforms \
                 requires the shared Rust core crate",
            ),
            (Self::MissingCore, PlatformsSurface::TopologySlot { registry, .. }) => {
                Error::validation_failed(
                    "topology-cache-project-platforms-must-include-core",
                    format!("workspace slot `{registry}` platform set includes `core`"),
                    format!(
                        "workspace slot `{registry}` platform set must include `core`; every \
                         project that declares platforms requires the shared Rust core crate",
                    ),
                )
            }
            (Self::NotAllowed { platform, allowed }, PlatformsSurface::Init { target }) => {
                Error::validation_failed(
                    "project-platforms-not-allowed",
                    format!("platform `{platform}` is not in the target's allowed set"),
                    format!(
                        "platform `{platform}` is not allowed by target '{target}'; allowed: [{}]",
                        allowed.join(", "),
                    ),
                )
            }
            (
                Self::NotAllowed { platform, allowed },
                PlatformsSurface::TopologySlot { registry, target },
            ) => Error::validation_failed(
                "topology-cache-project-platforms-not-allowed",
                format!("workspace slot `{registry}` platform `{platform}` is allowed"),
                format!(
                    "workspace slot `{registry}` platform `{platform}` is not allowed by target \
                     '{target}'; allowed: [{}]",
                    allowed.join(", "),
                ),
            ),
        }
    }
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
    pub(crate) fn check(&self, platforms: &[Platform]) -> Result<(), PlatformsViolation> {
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
/// the single `.wasm` component file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterLocation {
    /// Resolved from the global content-addressed adapter store entry
    /// at `<store-root>/<name>@<version>.wasm` — the immutable,
    /// version-keyed install target resolved through the carried
    /// `Locations` and populated by the wasm-pkg transport. Probed
    /// whenever the selector carries a pinned version.
    Store(PathBuf),
    /// Resolved from the project component cache
    /// (`<project-cache>/components/<name>.wasm`) — the seeded mirror
    /// `specify adapter add` (or a local-component init) populated.
    /// Probed for bare-name (unpinned) references and persisted
    /// component selectors; never outside the carried cache placement.
    Cache(PathBuf),
}

impl AdapterLocation {
    /// Kebab-case label for JSON envelopes (`"store"` / `"cache"`).
    #[must_use]
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Store(_) => "store",
            Self::Cache(_) => "cache",
        }
    }

    /// The component file path.
    #[must_use]
    pub const fn path(&self) -> &PathBuf {
        match self {
            Self::Store(path) | Self::Cache(path) => path,
        }
    }
}

/// Deployment-neutral description of where an adapter resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Resolver-defined mechanism label (`store`, `dev`, `native`, …).
    pub label: String,
    /// Human-readable reference to the resolved implementation.
    pub reference: String,
}

impl AdapterLocation {
    pub(super) fn origin(&self) -> Origin {
        Origin {
            label: self.label().to_string(),
            reference: self.path().display().to_string(),
        }
    }
}

/// In-memory identity + metadata of a resolved source adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAdapter {
    /// Kebab-case adapter name from the resolved identity.
    pub name: String,
    /// Semver adapter version: the pin for store-resolved (and
    /// native-catalog) identities; `None` for an unpinned cache
    /// resolve — a seeded component carries no package identity.
    pub version: Option<semver::Version>,
    /// Optional host-CLI compatibility floor from the metadata
    /// answer's `specify-floor`. The resolver compares it against the
    /// running binary (`check_requires_specify`) and aborts with
    /// `adapter-cli-too-old` (exit 3) when the binary is older.
    pub requires_specify: Option<semver::Version>,
}

/// In-memory identity + metadata of a resolved target adapter.
///
/// Constructed by a [`crate::adapter::Resolver`]: `name`/`version` from
/// the resolved selector identity, the rest from its metadata answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAdapter {
    /// Kebab-case adapter name from the resolved identity.
    pub name: String,
    /// Semver adapter version: the pin for store-resolved (and
    /// native-catalog) identities; `None` for an unpinned cache
    /// resolve — a seeded component carries no package identity.
    pub version: Option<semver::Version>,
    /// Optional host-CLI compatibility floor from the metadata
    /// answer's `specify-floor`. The resolver compares it against the
    /// running binary (`check_requires_specify`) and aborts with
    /// `adapter-cli-too-old` (exit 3) when the binary is older.
    pub requires_specify: Option<semver::Version>,
    /// Adapter-declared build inputs from the metadata answer. Each
    /// entry is a path relative to the build request's `inputs.root`,
    /// flagged `required`; the guest build orchestrator assembles
    /// `inputs.artifacts.additional[]` from this list.
    pub inputs: Vec<BuildInputDeclaration>,
    /// Optional platforms capability from the metadata answer. When
    /// present the target declares the closed set of [`Platform`]
    /// tokens it accepts, whether projects must declare platforms, and
    /// the default set for greenfield scaffolding.
    pub platforms: Option<PlatformsCapability>,
}

/// A resolved [`SourceAdapter`] paired with its deployment-neutral
/// origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    /// Resolved identity and metadata.
    pub manifest: SourceAdapter,
    /// Deployment-neutral implementation origin.
    pub origin: Origin,
}

/// A resolved [`TargetAdapter`] paired with its deployment-neutral
/// origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    /// Resolved identity and metadata.
    pub manifest: TargetAdapter,
    /// Deployment-neutral implementation origin.
    pub origin: Origin,
}

impl SourceAdapter {
    /// Iterator over the source operations this adapter serves, in
    /// ascending kebab-name order (`extract < survey`) — the axis's
    /// closed WIT operation set (`wit/specify.wit`).
    pub(crate) fn operations() -> impl Iterator<Item = &'static SourceOperation> {
        const WIT_OPERATIONS: &[SourceOperation] =
            &[SourceOperation::Extract, SourceOperation::Survey];
        WIT_OPERATIONS.iter()
    }
}

impl TargetAdapter {
    /// Iterator over the target operations this adapter serves, in
    /// ascending kebab-name order (`build < guidance < merge`) — the
    /// axis's closed WIT operation set (`wit/specify.wit`:
    /// guidance/build/merge).
    pub(crate) fn operations() -> impl Iterator<Item = &'static TargetOperation> {
        const WIT_OPERATIONS: &[TargetOperation] =
            &[TargetOperation::Build, TargetOperation::Guidance, TargetOperation::Merge];
        WIT_OPERATIONS.iter()
    }
}

/// Parse a metadata answer's `specify-floor` string into a typed
/// semver, naming the identity and resolved origin on failure.
///
/// # Errors
///
/// Returns [`Error::Validation`] with the kebab discriminant
/// `adapter-floor-malformed` when the floor is not exact semver.
pub(super) fn parse_floor(
    floor: Option<&str>, name: &str, origin: &Origin,
) -> Result<Option<semver::Version>, Error> {
    let Some(floor) = floor else {
        return Ok(None);
    };
    semver::Version::parse(floor).map(Some).map_err(|err| {
        Error::validation_failed(
            "adapter-floor-malformed",
            "an adapter's metadata answer declares a semver `specify-floor`",
            format!(
                "adapter `{name}` ({}) declares `specify-floor: {floor}`, which is not an exact semver: {err}",
                origin.reference,
            ),
        )
    })
}

/// Enforce an adapter's host-CLI compatibility floor.
///
/// `floor` is the adapter's optional `specify` minimum from its
/// metadata answer (already parsed into a typed `semver::Version`);
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
    floor: Option<&semver::Version>, current: &str, name: &str, origin: &Origin,
) -> Result<(), Error> {
    let Some(floor) = floor else {
        return Ok(());
    };
    let Ok(current_version) = semver::Version::parse(current) else {
        return Ok(());
    };
    if current_version < *floor {
        return Err(Error::AdapterCliTooOld {
            adapter: format!("{name} ({})", origin.reference),
            required: floor.to_string(),
            found: current.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unparseable_permissive() {
        let origin = Origin {
            label: "store".to_string(),
            reference: "/store/demo@1.0.0.wasm".to_string(),
        };
        let floor = semver::Version::new(2, 0, 0);

        check_requires_specify(Some(&floor), "not-a-version", "demo-target", &origin)
            .expect("an unparseable running version must not brick resolution");
    }
}
