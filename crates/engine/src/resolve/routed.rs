//! [`RoutedId`] — the typed routed adapter identity.
//!
//! One exact, opaque id (`<axis>:<name>[@<version>]`) names every seam
//! dispatch; this kernel is the single formatter/parser for that grammar.

use std::str::FromStr;

use emery_error::Error;

use super::core::Axis;
use super::selector::AdapterSelector;

/// One routed adapter identity: axis, kebab-case name, and the exact
/// SemVer pin a package-resolved identity carries (`None` for a
/// cache-backed resolve, which has no package identity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedId {
    /// Adapter axis (`source` / `target`).
    pub axis: Axis,
    /// Kebab-case adapter name.
    pub name: String,
    /// Exact SemVer pin; `None` routes the unversioned id.
    pub version: Option<semver::Version>,
}

impl RoutedId {
    /// A routed identity from its parts.
    #[must_use]
    pub fn new(axis: Axis, name: impl Into<String>, version: Option<semver::Version>) -> Self {
        Self {
            axis,
            name: name.into(),
            version,
        }
    }

    /// The routed identity implied by a recorded adapter value
    /// (`omnia@1.0.0` → `target:omnia@1.0.0`, `omnia` →
    /// `target:omnia`, `file://…/emery_omnia.wasm` →
    /// `target:omnia`). Total over historical values — an unparseable
    /// value routes the raw string as an unversioned name, mirroring
    /// [`AdapterSelector::recorded_name`].
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

    /// Parse a routed id string (`source:intent`,
    /// `target:omnia@1.0.0`).
    ///
    /// # Errors
    ///
    /// `adapter-routed-id-malformed` when the axis prefix, name, or
    /// version pin does not fit the grammar.
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
