//! Adapter identity
//!
//! An adapter is known by a `name@version` pair — the name it is addressed
//! by and the exact version it was published as. [`AdapterIdentity`] is the
//! parsed form, so code that needs the name or the version alone never has to
//! split the string itself.

use std::str::FromStr;

/// Adapter catalog identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterIdentity {
    /// Kebab-case adapter name.
    pub name: String,
    /// Exact SemVer version.
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
