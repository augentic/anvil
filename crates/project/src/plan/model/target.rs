//! Target concerns: the resolved `<name>[@<semver>]` target reference.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Parsed `<name>[@<semver>]` target-adapter identifier.
///
/// The *resolved* adapter form `name[@<semver>]` used by slice
/// metadata and `$TARGET`. Stored topology lives on
/// [`super::TargetBinding`].
/// Wire form is the single kebab string `name[@<semver>]`.
/// Deserialisation goes through `TargetRef::parse` and components are
/// private, so every value satisfies the wire grammar by construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetRef {
    name: String,
    version: Option<semver::Version>,
}

impl TargetRef {
    /// Parse a wire-form `<name>[@<semver>]` string.
    ///
    /// # Errors
    ///
    /// Returns [`TargetRefParseError`] when the string does not match
    /// the wire regex `^[a-z][a-z0-9-]*(@<semver>)?$` — empty segment,
    /// mixed case, non-semver version after an `@`, etc.
    pub(crate) fn parse(input: &str) -> Result<Self, TargetRefParseError> {
        let (name, version) = match input.split_once('@') {
            Some((name, version_part)) => {
                let version = semver::Version::parse(version_part)
                    .map_err(|_err| TargetRefParseError::new(input))?;
                (name, Some(version))
            }
            None => (input, None),
        };
        if !crate::name::is_kebab_leading_alpha(name) {
            return Err(TargetRefParseError::new(input));
        }
        Ok(Self {
            name: name.to_string(),
            version,
        })
    }
}

impl fmt::Display for TargetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(version) => write!(f, "{}@{version}", self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

impl Serialize for TargetRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for TargetRef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// Error returned by `TargetRef::parse` when the input does not
/// match the `name[@<semver>]` wire form.
///
/// Carries the offending input verbatim so callers can surface it in
/// diagnostics without re-formatting; the [`fmt::Display`] body is
/// already the kebab discriminant prose used by
/// `plan-target-malformed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRefParseError {
    /// The original (rejected) input.
    pub input: String,
}

impl TargetRefParseError {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
        }
    }
}

impl fmt::Display for TargetRefParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "target `{}` is not of the form `<name>` or `<name>@<semver>` (kebab name, exact \
             semver version when pinned)",
            self.input,
        )
    }
}

impl std::error::Error for TargetRefParseError {}
