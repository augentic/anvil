//! Plugin loading
//!
//! Loads plugins from the registry or local filesystem.

use std::path::{Path, PathBuf};
use std::str::FromStr;

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
        &self, selector: &AdapterRef, pin: Option<&Digest>, registry: Option<&str>,
    ) -> Result<Loaded, Error> {
        let name = selector.name()?;

        let loaded = match selector {
            AdapterRef::Package {
                namespace, version, ..
            } => {
                let request = PluginRef::builder()
                    .package(format!("{namespace}:{name}@{version}"))
                    .location(Location::Registry(registry.map(ToOwned::to_owned)))
                    .maybe_digest(pin.cloned())
                    .build();
                let plugin = Plugins::load(&self.cache, &request).await?;

                Loaded {
                    id: plugin.id().to_owned(),
                    digest: Some(plugin.digest().clone()),
                }
            }
            AdapterRef::Component(path) => {
                let id = format!("source:{name}");
                let digest = self.load_file(&id, path, pin).await?;

                Loaded {
                    id,
                    digest: Some(digest),
                }
            }
            AdapterRef::Bare(_) => Loaded {
                id: format!("source:{name}"),
                digest: None,
            },
        };

        self.check_version(&name, &loaded.id)?;

        Ok(loaded)
    }

    // The loader reads the file fresh — nothing is mirrored, so a deleted
    // file refuses on the next run. The engine keeps only the operator-typo
    // gate: a missing or non-component path refuses typed before any load.
    async fn load_file(
        &self, id: &str, path: &Path, pin: Option<&Digest>,
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
        let plugin = Plugins::load(&self.cache, &request).await?;

        Ok(plugin.digest().clone())
    }

    // Refuse when the running emery is older than the adapter's minimum.
    fn check_version(&self, name: &str, id: &str) -> Result<(), Error> {
        let metadata = Source::metadata(self.provider, id);
        let Some(adapter_version) = metadata.emery_version.as_deref() else {
            return Ok(());
        };
        let adapter_semver = semver::Version::parse(adapter_version).map_err(|err| {
            bad_request!(
                "adapter `{name}` ({id}) has an invalid `emery-version` `{adapter_version}`: {err}"
            )
        })?;

        let emery_semver = semver::Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("CARGO_PKG_VERSION is valid semver");
        if emery_semver < adapter_semver {
            return Err(Error::BadRequest {
                code: "unsupported-version".into(),
                description: format!(
                    "adapter {name} ({id}) requires emery {adapter_semver} or newer"
                ),
            });
        }

        Ok(())
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

/// An operator-supplied adapter reference.
#[derive(Debug, Clone)]
pub enum AdapterRef {
    /// Package reference (`emery:omnia@1.0.0`, `omnia@1.0.0`, etc.).
    Package {
        /// Kebab-case package namespace (`emery` for the shorthand).
        namespace: String,
        /// Kebab-case adapter name.
        name: String,
        /// Mandatory exact SemVer pin.
        version: semver::Version,
    },
    /// Bare adapter name (`omnia`): the kebab-case adapter name.
    Bare(String),
    /// Local component file path (`./intent.wasm`).
    Component(PathBuf),
}

impl FromStr for AdapterRef {
    type Err = Error;

    /// Parses an adapter from a string. Valid strings are:
    ///   - `emery:intent@1.0.0`
    ///   - `intent@1.0.0`
    ///   - `intent`
    ///   - `./intent.wasm`
    ///
    /// # Errors
    ///
    /// Returns typed errors for malformed values, GitHub URLs, or invalid pins.
    fn from_str(value: &str) -> Result<Self, Error> {
        let value = value.trim();
        if value.is_empty() {
            return Err(bad_request!("adapter reference is empty"));
        }
        if value.starts_with("https://github.com/") {
            return Err(bad_request!("adapter `{value}`: GitHub URLs are not supported"));
        }

        if let Some((namespace, rest)) = value.split_once(':') {
            // URL authorities and Windows drive paths are not package references.
            if !rest.starts_with('/') && is_kebab(namespace) {
                return Self::new(namespace, rest, value);
            }
        }

        match value.split_once('@') {
            // `<name>@<version>` is sugar for the `emery` namespace; a
            // non-SemVer suffix falls through to the path grammar.
            Some((name, version)) if is_kebab(name) => {
                if let Ok(version) = semver::Version::parse(version) {
                    return Ok(Self::Package {
                        namespace: "emery".to_string(),
                        name: name.to_string(),
                        version,
                    });
                }
            }
            None if is_kebab(value) => {
                return Ok(Self::Bare(value.to_string()));
            }
            Some(_) | None => {}
        }

        let path = value.strip_prefix("file://").unwrap_or(value);
        Ok(Self::Component(PathBuf::from(path)))
    }
}

impl AdapterRef {
    fn new(namespace: &str, rest: &str, original: &str) -> Result<Self, Error> {
        let (name, version) = rest
            .split_once('@')
            .ok_or_else(|| bad_request!("adapter `{original}` is missing `@<version>`"))?;
        if name.is_empty() {
            return Err(bad_request!("adapter `{original}` is missing a name before `@`"));
        }

        let version = semver::Version::parse(version).map_err(|err| {
            bad_request!("adapter `{original}` has an invalid version `{version}`: {err}")
        })?;

        Ok(Self::Package {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version,
        })
    }

    /// Returns the kebab-case adapter name.
    ///
    /// # Errors
    ///
    /// Returns a `BadRequest` for an unusable component stem.
    pub fn name(&self) -> Result<String, Error> {
        match self {
            Self::Bare(name) | Self::Package { name, .. } => Ok(name.clone()),
            Self::Component(path) => {
                let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or_else(|| {
                    let path = path.display();
                    bad_request!("cannot derive adapter name from {path}")
                })?;
                let stem = stem
                    .strip_prefix("emery_")
                    .or_else(|| stem.strip_prefix("emery-"))
                    .unwrap_or(stem);
                Ok(stem.replace('_', "-"))
            }
        }
    }
}

// Equivalent to `^[a-z][a-z0-9-]*$`.
fn is_kebab(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
