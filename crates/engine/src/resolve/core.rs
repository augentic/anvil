//! Adapter identity and post-resolution compatibility gates.

use emery_error::Error;
use serde::{Deserialize, Serialize};

/// Adapter component axis.
///
/// `Target` remains parseable for typed refusals but is not live.
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
    /// Source adapter.
    Source,
    /// Deferred target adapter.
    Target,
}

impl Axis {
    /// Returns the deployment and prose directory segment.
    #[must_use]
    pub const fn dir_segment(self) -> &'static str {
        match self {
            Self::Source => "sources",
            Self::Target => "targets",
        }
    }

    /// Returns the routed-id prefix.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Target => "target",
        }
    }
}

/// Engine storage location of an adapter component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterLocation {
    /// Immutable object in the global adapter store.
    Store(String),
    /// Seeded object in the project component cache.
    Cache(String),
}

impl AdapterLocation {
    /// Returns the location label.
    #[must_use]
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Store(_) => "store",
            Self::Cache(_) => "cache",
        }
    }

    /// Returns the component object name.
    #[must_use]
    pub fn object(&self) -> &str {
        match self {
            Self::Store(object) | Self::Cache(object) => object,
        }
    }

    /// Returns the location's blobstore container.
    pub(super) const fn container(&self) -> &'static str {
        match self {
            Self::Store(_) => crate::handler::STORE_CONTAINER,
            Self::Cache(_) => crate::handler::ADAPTERS_CONTAINER,
        }
    }

    /// Returns a deployment-neutral origin.
    pub(super) fn origin(&self) -> Origin {
        Origin {
            label: self.label().to_string(),
            reference: format!("{}/{}", self.container(), self.object()),
        }
    }
}

/// Deployment-neutral adapter origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Resolver mechanism label.
    pub label: String,
    /// Human-readable reference to the resolved implementation.
    pub reference: String,
}

/// Identity and metadata of a resolved source adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAdapter {
    /// Kebab-case adapter name.
    pub name: String,
    /// Package version, absent for unpinned cache resolutions.
    pub version: Option<semver::Version>,
    /// Optional Emery CLI compatibility floor.
    pub requires_emery: Option<semver::Version>,
}

/// A resolved source adapter and its origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    /// Resolved identity and metadata.
    pub manifest: SourceAdapter,
    /// Deployment-neutral implementation origin.
    pub origin: Origin,
}

/// Parses an adapter's Emery compatibility floor.
///
/// # Errors
///
/// Returns `adapter-floor-malformed` for non-exact SemVer.
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

/// Enforces an adapter's Emery CLI compatibility floor.
///
/// An unparseable running version is permissive to preserve recovery.
///
/// # Errors
///
/// Returns [`Error::AdapterCliTooOld`] when `current` is below `floor`.
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
