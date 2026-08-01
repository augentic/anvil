//! Axis-split adapter identity model and post-resolve coherence gates.
//!
//! An adapter is a single WebAssembly component: identity
//! lives in the wasm-pkg package reference (`emery:<name>@<semver>`),
//! axis in the exported world (`source` xor `target`), and the
//! remaining metadata (compatibility floor, build inputs, platforms
//! in the component's own deterministic `metadata` export
//! (see [`super::metadata`]). There is no on-disk manifest.
//!
//! Resolution lives in [`super::resolve`]; this module owns the typed
//! identity structs, the location enum, and the post-resolve floor
//! gate ([`check_requires_emery`]).

use std::path::PathBuf;

use error::Error;
use serde::{Deserialize, Serialize};

use crate::adapter::operation::{SourceOperation, TargetOperation};

mod platforms;

pub use platforms::{PlatformsCapability, PlatformsSurface};

/// Axis discriminator for an adapter component.
///
/// Source vs target — see workflow §Adapter vocabulary. The closed enum
/// routes the resolver dispatcher (`commands::resolve_adapter`) and the
/// metadata dispatch; the in-memory adapters themselves are axis-typed
/// ([`SourceAdapter`] / [`TargetAdapter`]) so internal call sites no
/// longer carry the `axis` argument forward past the resolver boundary.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
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
    /// `emery adapter add` (or a local-component init) populated.
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
    /// answer's `emery-floor`. The resolver compares it against the
    /// running binary (`check_requires_emery`) and aborts with
    /// `adapter-cli-too-old` (exit 3) when the binary is older.
    pub requires_emery: Option<semver::Version>,
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
    /// answer's `emery-floor`. The resolver compares it against the
    /// running binary (`check_requires_emery`) and aborts with
    /// `adapter-cli-too-old` (exit 3) when the binary is older.
    pub requires_emery: Option<semver::Version>,
    /// Adapter-declared build inputs from the metadata answer. Each
    /// entry is a path relative to the build request's `inputs.root`,
    /// flagged `required`; the guest build orchestrator assembles
    /// `inputs.artifacts.additional[]` from this list.
    pub inputs: Vec<BuildInputDeclaration>,
    /// Optional platforms capability from the metadata answer. When
    /// present the target declares the closed set of [`crate::Platform`]
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
    /// closed WIT operation set (`wit/emery.wit`).
    pub(crate) fn operations() -> impl Iterator<Item = &'static SourceOperation> {
        const WIT_OPERATIONS: &[SourceOperation] =
            &[SourceOperation::Extract, SourceOperation::Survey];
        WIT_OPERATIONS.iter()
    }
}

impl TargetAdapter {
    /// Iterator over the target operations this adapter serves, in
    /// ascending kebab-name order (`build < guidance < merge`) — the
    /// axis's closed WIT operation set (`wit/emery.wit`:
    /// guidance/build/merge).
    pub(crate) fn operations() -> impl Iterator<Item = &'static TargetOperation> {
        const WIT_OPERATIONS: &[TargetOperation] =
            &[TargetOperation::Build, TargetOperation::Guidance, TargetOperation::Merge];
        WIT_OPERATIONS.iter()
    }
}

/// Parse a metadata answer's `emery-floor` string into a typed
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
            "an adapter's metadata answer declares a semver `emery-floor`",
            format!(
                "adapter `{name}` ({}) declares `emery-floor: {floor}`, which is not an exact semver: {err}",
                origin.reference,
            ),
        )
    })
}

/// Enforce an adapter's host-CLI compatibility floor.
///
/// `floor` is the adapter's optional `emery` minimum from its
/// metadata answer (already parsed into a typed `semver::Version`);
/// `current` is the running binary's version (the resolve call sites
/// pass `env!("CARGO_PKG_VERSION")`, the same source [`crate::config`]
/// uses). When the binary is older than the floor the adapter cannot be
/// honored, so resolution aborts with [`Error::AdapterCliTooOld`] on
/// the exit-3 `EXIT_VERSION_TOO_OLD` path — the adapter-granularity
/// analog of the `project.yaml` `emery` floor.
///
/// `current` is parsed permissively: an unparseable running version is
/// treated as "not older" rather than bricking resolution, mirroring
/// `config::version_is_older`. An absent `floor` is a clean pass.
///
/// # Errors
///
/// Returns [`Error::AdapterCliTooOld`] when `current` parses below
/// `floor`.
pub(super) fn check_requires_emery(
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

        check_requires_emery(Some(&floor), "not-a-version", "demo-target", &origin)
            .expect("an unparseable running version must not brick resolution");
    }
}
