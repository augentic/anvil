//! Parsed adapter identity (`name@version`).

use std::str::FromStr;

/// Catalog identity: kebab `name` plus exact SemVer `version`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdapterIdentity {
    /// Kebab-case adapter name.
    pub name: String,
    /// Exact SemVer version string.
    pub version: String,
}

/// Invalid `name@version` adapter identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("adapter identity must be `name@version`")]
pub struct IdentityError;

impl FromStr for AdapterIdentity {
    type Err = IdentityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((name, version)) = s.split_once('@') else {
            return Err(IdentityError);
        };
        if name.is_empty() || version.is_empty() || version.contains('@') {
            return Err(IdentityError);
        }
        Ok(Self {
            name: name.to_string(),
            version: version.to_string(),
        })
    }
}
