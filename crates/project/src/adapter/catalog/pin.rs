//! Exact package pin (`emery:<name>@<semver>`). Detached topology
//! refuses bare names and local components.

use error::Error;

use crate::adapter::{AdapterSelector, FIRST_PARTY_NAMESPACE};

/// Exact adapter package pin recorded on a detached binding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pin {
    /// Registry namespace (`emery` for first-party adapters).
    pub namespace: String,
    /// Kebab-case adapter name.
    pub name: String,
    /// Exact SemVer pin.
    pub version: semver::Version,
}

impl Pin {
    /// First-party pin `emery:<name>@<version>`.
    #[must_use]
    pub fn emery(name: impl Into<String>, version: semver::Version) -> Self {
        Self {
            namespace: FIRST_PARTY_NAMESPACE.to_string(),
            name: name.into(),
            version,
        }
    }

    /// Parse an exact package pin. Bare names and component paths fail.
    ///
    /// # Errors
    ///
    /// `adapter-unversioned` for a bare name or local component;
    /// `adapter-arg-malformed` and the package-ref codes from
    /// [`AdapterSelector::parse`].
    pub fn parse(raw: &str) -> Result<Self, Error> {
        match AdapterSelector::parse(raw)? {
            AdapterSelector::Package {
                namespace,
                name,
                version,
            } => Ok(Self {
                namespace,
                name,
                version,
            }),
            AdapterSelector::Bare { name } => Err(unversioned(format!(
                "adapter `{name}` is unversioned; detached topology requires `emery:<name>@<semver>`"
            ))),
            AdapterSelector::Component { path } => Err(unversioned(format!(
                "local component `{}` cannot enter detached topology; pin `emery:<name>@<semver>`",
                path.display()
            ))),
        }
    }

    /// Canonical wire form `<namespace>:<name>@<version>`.
    #[must_use]
    pub fn wire(&self) -> String {
        format!("{}:{}@{}", self.namespace, self.name, self.version)
    }
}

impl std::fmt::Display for Pin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.wire())
    }
}

pub(super) const fn unversioned(detail: String) -> Error {
    Error::Diag {
        code: "adapter-unversioned",
        detail,
    }
}
