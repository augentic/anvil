//! Kind-preserving adapter references and parsing.

use std::path::{Path, PathBuf};

use emery_error::Error as Legacy;

use crate::handler::{Error, classify};

/// An operator-supplied adapter reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterSelector {
    /// Bare unpinned shorthand (`omnia`).
    Bare {
        /// Kebab-case adapter name.
        name: String,
    },
    /// Exact package reference (`emery:omnia@1.0.0`; `omnia@1.0.0`
    /// is sugar for the `emery` namespace).
    Package {
        /// Kebab-case adapter name.
        name: String,
        /// Mandatory exact SemVer pin.
        version: semver::Version,
    },
    /// Local component file path.
    Component {
        /// Supplied path, anchored at the project directory when relative.
        path: PathBuf,
    },
}

impl AdapterSelector {
    /// Parses an adapter argument without filesystem access.
    ///
    /// # Errors
    ///
    /// Returns typed errors for malformed values, GitHub URLs, or invalid pins.
    pub fn parse(value: &str) -> Result<Self, Error> {
        if value.trim().is_empty() || value != value.trim() {
            return Err(classify(&Legacy::Diag {
                code: "adapter-arg-malformed",
                detail:
                    "<adapter> must be non-empty and must not have leading or trailing whitespace"
                        .to_string(),
            }));
        }
        if is_github_url(value) {
            return Err(classify(&Legacy::Diag {
                code: "adapter-github-uri-unsupported",
                detail: format!(
                    "GitHub adapter URIs are not supported (`{value}`): a source checkout \
                     does not yield a usable adapter artifact. Pin a published component \
                     (`emery:<name>@<semver>`) or point at a local `.wasm` component file"
                ),
            }));
        }
        if let Some(package) = recognize_package(value) {
            return package;
        }
        if let Some((name, version)) = parse_shorthand(value) {
            return Ok(version.map_or_else(
                || Self::Bare {
                    name: name.to_string(),
                },
                |version| Self::Package {
                    name: name.to_string(),
                    version,
                },
            ));
        }
        let path = value.strip_prefix("file://").unwrap_or(value);
        Ok(Self::Component {
            path: PathBuf::from(path),
        })
    }

    /// Returns the kebab-case adapter name.
    ///
    /// # Errors
    ///
    /// Returns `adapter-dir-name-unresolved` for an unusable component stem.
    pub fn name(&self) -> Result<String, Error> {
        match self {
            Self::Bare { name } | Self::Package { name, .. } => Ok(name.clone()),
            Self::Component { path } => name_from_component(path),
        }
    }

    /// Returns the exact package pin, if present.
    #[must_use]
    pub const fn version(&self) -> Option<&semver::Version> {
        match self {
            Self::Package { version, .. } => Some(version),
            Self::Bare { .. } | Self::Component { .. } => None,
        }
    }
}

fn is_github_url(value: &str) -> bool {
    value.starts_with("https://github.com/")
}

// `None` means another selector grammar may handle the value.
fn recognize_package(value: &str) -> Option<Result<AdapterSelector, Error>> {
    let (namespace, rest) = value.split_once(':')?;
    // URL authorities and Windows drive paths are not package references.
    if rest.starts_with('/') || !is_first_party_name(namespace) {
        return None;
    }
    Some(parse_validated_package(namespace, rest, value))
}

fn parse_validated_package(
    namespace: &str, rest: &str, original: &str,
) -> Result<AdapterSelector, Error> {
    let (name, version) = rest.split_once('@').ok_or_else(|| {
        classify(&Legacy::Diag {
            code: "adapter-package-ref-version-required",
            detail: format!(
                "adapter package reference `{original}` must pin an exact SemVer version (`{namespace}:<name>@<version>`); there is no branch or tag defaulting"
            ),
        })
    })?;
    if name.is_empty() {
        return Err(classify(&Legacy::Diag {
            code: "adapter-package-ref-malformed",
            detail: format!(
                "adapter package reference `{original}` is missing a package name before `@`"
            ),
        }));
    }
    let version = semver::Version::parse(version).map_err(|err| {
        classify(&Legacy::Diag {
            code: "adapter-package-ref-version-required",
            detail: format!(
                "adapter package reference `{original}` must pin an exact SemVer version, not `{version}`: {err}"
            ),
        })
    })?;
    Ok(AdapterSelector::Package {
        name: name.to_string(),
        version,
    })
}

// `None` lets the component-path grammar handle the value.
fn parse_shorthand(value: &str) -> Option<(&str, Option<semver::Version>)> {
    if value.contains('/') || value.contains(':') {
        return None;
    }
    match value.split_once('@') {
        Some((name, reference)) if is_first_party_name(name) => {
            let version = semver::Version::parse(reference).ok()?;
            Some((name, Some(version)))
        }
        None if is_first_party_name(value) => Some((value, None)),
        Some(_) | None => None,
    }
}

// Equivalent to `^[a-z][a-z0-9-]*$`.
fn is_first_party_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Derives a kebab-case adapter name from a component filename.
///
/// # Errors
///
/// Returns `adapter-dir-name-unresolved` for an unusable stem.
pub fn name_from_component(path: &Path) -> Result<String, Error> {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or_else(|| {
        classify(&Legacy::Diag {
            code: "adapter-dir-name-unresolved",
            detail: format!("cannot derive adapter name from {}", path.display()),
        })
    })?;
    let stem = stem.strip_prefix("emery_").or_else(|| stem.strip_prefix("emery-")).unwrap_or(stem);
    Ok(stem.replace('_', "-"))
}
