//! Routed adapter identity parsing and formatting.

use std::str::FromStr;

use emery_error::Error;

use super::core::Axis;
use super::selector::AdapterSelector;

/// An adapter identity used for seam dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedId {
    /// Adapter axis (`source` / `target`).
    pub axis: Axis,
    /// Kebab-case adapter name.
    pub name: String,
    /// Exact package pin; absent for unversioned dispatch.
    pub version: Option<semver::Version>,
}

impl RoutedId {
    /// Creates a routed identity.
    #[must_use]
    pub fn new(axis: Axis, name: impl Into<String>, version: Option<semver::Version>) -> Self {
        Self {
            axis,
            name: name.into(),
            version,
        }
    }

    /// Derives an identity from a recorded value.
    ///
    /// Malformed historical values route as raw unversioned names.
    #[must_use]
    pub fn recorded(axis: Axis, value: &str) -> Self {
        AdapterSelector::parse(value)
            .ok()
            .and_then(|selector| {
                let version = selector.version().cloned();
                selector.name().ok().map(|name| Self::new(axis, name, version))
            })
            .unwrap_or_else(|| Self::new(axis, value, None))
    }

    /// Parses `<axis>:<name>[@<version>]`.
    ///
    /// # Errors
    ///
    /// Returns `adapter-routed-id-malformed` for invalid grammar.
    pub fn parse(value: &str) -> Result<Self, Error> {
        let malformed = |detail: String| Error::Diag {
            code: "adapter-routed-id-malformed",
            detail,
        };
        let (axis, rest) = value.split_once(':').ok_or_else(|| {
            malformed(format!(
                "routed adapter id `{value}` is missing its `<axis>:` prefix (`source:` or \
                 `target:`)"
            ))
        })?;
        let axis = Axis::from_str(axis).map_err(|_unknown_variant| {
            malformed(format!(
                "routed adapter id `{value}` names axis `{axis}`; expected `source` or `target`"
            ))
        })?;
        let (name, version) = match rest.split_once('@') {
            Some((name, version)) => {
                let version = semver::Version::parse(version).map_err(|err| {
                    malformed(format!(
                        "routed adapter id `{value}` pins version `{version}`, which is not \
                         exact SemVer: {err}"
                    ))
                })?;
                (name, Some(version))
            }
            None => (rest, None),
        };
        if name.is_empty() {
            return Err(malformed(format!("routed adapter id `{value}` is missing its name")));
        }
        Ok(Self::new(axis, name, version))
    }
}

impl std::fmt::Display for RoutedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.version {
            Some(version) => write!(f, "{}:{}@{version}", self.axis.prefix(), self.name),
            None => write!(f, "{}:{}", self.axis.prefix(), self.name),
        }
    }
}
