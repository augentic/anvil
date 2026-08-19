//! Adapter identity model and post-resolve coherence gates.
//!
//! Identity lives in the package reference, axis in the exported world;
//! there is no on-disk manifest. Only the source axis is live (ADR-0008).

use std::path::PathBuf;

use error::Error;
use serde::{Deserialize, Serialize};

/// Axis discriminator for an adapter component.
///
/// Only [`Axis::Source`] is live; `Target` survives in the routed-id
/// grammar so recorded target ids parse to a typed refusal rather than
/// a grammar error (the target axis returns with the build programme).
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
    /// Source adapter — `extract` + `metadata`.
    Source,
    /// Target adapter — deferred with the build programme.
    Target,
}

impl Axis {
    /// Axis segment used by deployment guest ids and prose trees.
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

/// Where an adapter component was located on disk. The carried path is
/// the single `.wasm` component file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterLocation {
    /// Resolved from the global content-addressed adapter store entry
    /// at `<store-root>/<name>@<version>.wasm` — the immutable,
    /// version-keyed install target resolved through the carried
    /// `Locations`. Probed whenever the selector carries a pinned
    /// version.
    Store(PathBuf),
    /// Resolved from the project component cache
    /// (`<project-cache>/components/<name>.wasm`) — the seeded mirror
    /// a local-component init populated. Probed for bare-name
    /// (unpinned) references and persisted component selectors; never
    /// outside the carried cache placement.
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

    pub(super) fn origin(&self) -> Origin {
        Origin {
            label: self.label().to_string(),
            reference: self.path().display().to_string(),
        }
    }
}

/// Deployment-neutral description of where an adapter resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Resolver-defined mechanism label (`store`, `cache`, `native`, …).
    pub label: String,
    /// Human-readable reference to the resolved implementation.
    pub reference: String,
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

/// A resolved [`SourceAdapter`] paired with its deployment-neutral
/// origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    /// Resolved identity and metadata.
    pub manifest: SourceAdapter,
    /// Deployment-neutral implementation origin.
    pub origin: Origin,
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
/// When the running binary is older than the adapter's declared
/// `emery` floor, resolution aborts with [`Error::AdapterCliTooOld`]
/// on the exit-3 `EXIT_VERSION_TOO_OLD` path. `current` is parsed
/// permissively — an unparseable running version is treated as "not
/// older" rather than bricking resolution; an absent floor passes.
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

// Keep (CLI-unreachable defensive branch): production `current` is the
// binary's own always-parseable `env!("CARGO_PKG_VERSION")`, so no CLI
// input can reach the permissive unparseable-version arm.
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

        check_requires_emery(Some(&floor), "not-a-version", "demo-source", &origin)
            .expect("an unparseable running version must not brick resolution");
    }
}
