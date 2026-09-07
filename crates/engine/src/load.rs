//! Source-adapter loading: selector parsing, local components and
//! registry packages over the `Plugins` capability, plus the adapter
//! `emery-version` gate.

use std::path::{Path, PathBuf};

use emery_source::Source;
use omnia_guest::plugins::{Digest, Location, PluginCache, PluginRef};
use omnia_guest::{Error, Plugins, bad_request, not_found};

use crate::preopen_path;

/// One run's adapter loads over a provider: loads memoize by identity
/// for the run, so a second binding on the same adapter reuses the held
/// guest and a disagreeing pin refuses `already-active` from the memo
/// rather than the host.
pub struct Loader<'a, P: Plugins> {
    provider: &'a P,
    cache: PluginCache<&'a P>,
}

impl<'a, P: Source + Plugins> Loader<'a, P> {
    /// An empty memo over `provider`.
    pub const fn new(provider: &'a P) -> Self {
        Self {
            provider,
            cache: PluginCache::new(provider),
        }
    }

    /// Loads the adapter a selector names — a local component or registry
    /// package — enforcing its minimum `emery-version`, and returns its
    /// routed dispatch id.
    ///
    /// # Errors
    ///
    /// Returns selector, load, or version failures.
    pub async fn load(
        &self, selector: &AdapterSelector, pin: Option<&Digest>, registry: Option<&str>,
    ) -> Result<Loaded, Error> {
        let name = selector.name()?;

        // load the adapter to its routed dispatch id and digest
        let loaded = match selector {
            AdapterSelector::Package {
                namespace,
                name,
                version,
            } => {
                let request = PluginRef::builder()
                    .package(format!("{namespace}:{name}@{version}"))
                    .location(Location::Registry(registry.map(ToOwned::to_owned)))
                    .maybe_digest(pin.cloned())
                    .build();
                let plugin = Plugins::load(&self.cache, &request).await?;
                let digest = plugin.digest().clone();

                Loaded {
                    id: plugin.id().to_owned(),
                    digest: Some(digest),
                }
            }
            AdapterSelector::Component { path } => {
                let id = format!("source:{name}");
                let digest = load_component(&self.cache, &id, path, pin).await?;

                Loaded {
                    id,
                    digest: Some(digest),
                }
            }
            AdapterSelector::Bare { .. } => Loaded {
                id: format!("source:{name}"),
                digest: None,
            },
        };

        // check the adapter's minimum `emery-version`
        let metadata = Source::metadata(self.provider, &loaded.id);
        let minimum = parse_minimum(metadata.emery_version.as_deref(), &name, &loaded.id)?;
        check_minimum(minimum.as_ref(), env!("CARGO_PKG_VERSION"), &name, &loaded.id)?;

        Ok(loaded)
    }
}

/// One loaded source adapter: the routed dispatch id plus, for a
/// loader-loaded adapter, its content digest.
#[derive(Debug)]
pub struct Loaded {
    /// Routed dispatch id: the package reference for a registry
    /// package, `source:<name>` otherwise.
    pub id: String,
    /// Sha256 digest of the loaded component bytes.
    pub digest: Option<Digest>,
}

// The loader reads the file fresh — nothing is mirrored, so a deleted
// file refuses on the next run. The engine keeps only the operator-typo
// gate: a missing or non-component path refuses typed before any load.
async fn load_component<L: Plugins>(
    cache: &L, id: &str, path: &Path, pin: Option<&Digest>,
) -> Result<Digest, Error> {
    let relative = preopen_path(path)?;
    if !relative.is_file() || relative.extension().is_none_or(|ext| ext != "wasm") {
        let path = path.display();
        let relative = relative.display();

        return Err(not_found!(
            "adapter `{path}` did not resolve to a `.wasm` component at {relative}"
        ));
    }

    let request = PluginRef::builder()
        .package(id)
        .location(Location::Path(relative.display().to_string()))
        .maybe_digest(pin.cloned())
        .build();
    let plugin = Plugins::load(cache, &request).await?;

    Ok(plugin.digest().clone())
}

// Get the adapter's minimum `emery-version` from its metadata.
fn parse_minimum(
    minimum: Option<&str>, name: &str, id: &str,
) -> Result<Option<semver::Version>, Error> {
    let Some(minimum) = minimum else {
        return Ok(None);
    };

    semver::Version::parse(minimum).map(Some).map_err(|err| {
        bad_request!("adapter `{name}` ({id}) has an invalid `emery-version` `{minimum}`: {err}")
    })
}

// Check the adapter's minimum `emery-version` against the running version.
fn check_minimum(
    minimum: Option<&semver::Version>, current: &str, name: &str, id: &str,
) -> Result<(), Error> {
    let Some(minimum) = minimum else {
        return Ok(());
    };
    let Ok(version) = semver::Version::parse(current) else {
        return Ok(());
    };

    if version < *minimum {
        return Err(Error::BadRequest {
            code: "unsupported-version".into(),
            description: format!("adapter {name} ({id}) requires emery {minimum} or newer"),
        });
    }

    Ok(())
}

/// An operator-supplied adapter reference.
#[derive(Debug, Clone)]
pub enum AdapterSelector {
    /// Bare unpinned shorthand (`omnia`).
    Bare {
        /// Kebab-case adapter name.
        name: String,
    },
    /// Exact package reference (`emery:omnia@1.0.0`; `omnia@1.0.0`
    /// is sugar for the `emery` namespace).
    Package {
        /// Kebab-case package namespace (`emery` for the shorthand).
        namespace: String,
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
            return Err(bad_request!("adapter reference is empty or has surrounding whitespace"));
        }
        if value.starts_with("https://github.com/") {
            return Err(bad_request!(
                "adapter `{value}`: GitHub URLs are not supported; use `emery:<name>@<version>` \
                 or a local `.wasm` path"
            ));
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
                    namespace: "emery".to_string(),
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
    /// Returns a `BadRequest` for an unusable component stem.
    pub fn name(&self) -> Result<String, Error> {
        match self {
            Self::Bare { name } | Self::Package { name, .. } => Ok(name.clone()),
            Self::Component { path } => name_from_component(path),
        }
    }
}

// The kebab-case adapter name of a component filename.
fn name_from_component(path: &Path) -> Result<String, Error> {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or_else(|| {
        let path = path.display();
        bad_request!("cannot derive adapter name from {path}")
    })?;
    let stem = stem.strip_prefix("emery_").or_else(|| stem.strip_prefix("emery-")).unwrap_or(stem);
    Ok(stem.replace('_', "-"))
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
        bad_request!(
            "adapter `{original}` is missing `@<version>` (expected \
             `{namespace}:<name>@<version>`)"
        )
    })?;
    if name.is_empty() {
        return Err(bad_request!("adapter `{original}` is missing a name before `@`"));
    }
    let version = semver::Version::parse(version).map_err(|err| {
        bad_request!("adapter `{original}` has an invalid version `{version}`: {err}")
    })?;
    Ok(AdapterSelector::Package {
        namespace: namespace.to_string(),
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

// Keep (entry-point-unreachable defensive branch): production `current`
// is the binary's own always-parseable `env!("CARGO_PKG_VERSION")`, so
// no operator input can reach the permissive unparseable-version arm.
#[cfg(test)]
mod tests {
    use super::check_minimum;

    #[test]
    fn unparseable_permissive() {
        let minimum = semver::Version::new(2, 0, 0);

        check_minimum(Some(&minimum), "not-a-version", "demo-source", "source:demo-source")
            .expect("an unparseable running version must not brick loading");
    }
}
