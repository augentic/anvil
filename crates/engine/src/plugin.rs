//! Source-adapter loading over the `omnia:plugins/loader` seam: the typed
//! selector, the per-run load memo, and the adapter `emery-version` gate.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::LazyLock;

use emery_source::Source;
use omnia_guest::plugins::{Digest, Location, Plugin, PluginCache, PluginRef};
use omnia_guest::{Error, Plugins, bad_request, not_found};

use crate::preopen_path;

// The running emery, parsed once for the minimum-version gate.
static EMERY: LazyLock<semver::Version> = LazyLock::new(|| {
    env!("CARGO_PKG_VERSION").parse().expect("CARGO_PKG_VERSION is valid semver")
});

/// One run's adapter loads over a provider: loads memoize by identity
/// for the run, so a second binding on the same adapter reuses the held
/// guest and a disagreeing pin refuses `already-active` from the memo
/// rather than the host.
pub(crate) struct Loader<'a, P: Source + Plugins> {
    provider: &'a P,
    // The memo wraps the same provider; `PluginCache` exposes no accessor
    // for it, so the version gate keeps its own reference.
    cache: PluginCache<&'a P>,
}

impl<'a, P: Source + Plugins> Loader<'a, P> {
    /// An empty memo over `provider`.
    pub(crate) const fn new(provider: &'a P) -> Self {
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
    pub(crate) async fn load(
        &self, selector: &AdapterRef, pin: Option<&Digest>, registry: Option<&str>,
    ) -> Result<Loaded, Error> {
        let name = selector.name();
        let loaded = match selector.request(pin, registry)? {
            Some(request) => Loaded::from(self.cache.ensure(&request).await?),
            None => Loaded {
                id: format!("source:{name}"),
                digest: None,
            },
        };
        check_version(self.provider, name, &loaded.id)?;

        Ok(loaded)
    }
}

// Refuse when the running emery is older than the adapter's minimum.
fn check_version<P: Source>(provider: &P, name: &str, id: &str) -> Result<(), Error> {
    let Some(declared) = provider.metadata(id).emery_version else {
        return Ok(());
    };
    let minimum = semver::Version::parse(&declared).map_err(|err| {
        bad_request!("adapter `{name}` ({id}) has an invalid `emery-version` `{declared}`: {err}")
    })?;

    if *EMERY < minimum {
        return Err(Error::BadRequest {
            code: "unsupported-version".into(),
            description: format!("adapter {name} ({id}) requires emery {minimum} or newer"),
        });
    }

    Ok(())
}

/// One loaded source adapter: the routed dispatch id plus, for a
/// loader-loaded adapter, its content digest.
#[derive(Debug)]
pub(crate) struct Loaded {
    /// Routed dispatch id: the package reference for a registry
    /// package, `source:<name>` otherwise.
    pub(crate) id: String,
    /// Sha256 digest of the loaded component bytes.
    pub(crate) digest: Option<Digest>,
}

impl From<Plugin> for Loaded {
    fn from(plugin: Plugin) -> Self {
        Self {
            id: plugin.id().to_owned(),
            digest: Some(plugin.digest().clone()),
        }
    }
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
    /// Local component file (`./intent.wasm`).
    Component {
        /// Kebab-case adapter name derived from the file stem.
        name: String,
        /// Project-relative component path.
        path: PathBuf,
    },
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
    /// Returns typed errors for malformed values, GitHub URLs, invalid
    /// pins, or a component path with no usable stem.
    fn from_str(value: &str) -> Result<Self, Error> {
        let value = value.trim();
        if value.is_empty() {
            return Err(bad_request!("adapter reference is empty"));
        }
        if value.starts_with("https://github.com/") {
            return Err(bad_request!("adapter `{value}`: GitHub URLs are not supported"));
        }

        // URL authorities and Windows drive paths are not package references.
        if let Some((namespace, rest)) = value.split_once(':')
            && !rest.starts_with('/')
            && is_kebab(namespace)
        {
            return Self::package(namespace, rest, value);
        }

        match value.split_once('@') {
            // `<name>@<version>` is sugar for the `emery` namespace; a
            // non-SemVer suffix falls through to the path grammar.
            Some((name, _)) if is_kebab(name) => {
                if let Ok(package) = Self::package("emery", value, value) {
                    return Ok(package);
                }
            }
            None if is_kebab(value) => return Ok(Self::Bare(value.to_owned())),
            _ => {}
        }

        Self::component(value.strip_prefix("file://").unwrap_or(value))
    }
}

impl AdapterRef {
    /// Returns the kebab-case adapter name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bare(name) | Self::Package { name, .. } | Self::Component { name, .. } => name,
        }
    }

    fn package(namespace: &str, rest: &str, original: &str) -> Result<Self, Error> {
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
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            version,
        })
    }

    // The adapter name is the file stem, minus an `emery` crate prefix,
    // in kebab case.
    fn component(path: &str) -> Result<Self, Error> {
        let stem = Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| bad_request!("cannot derive adapter name from {path}"))?;
        let stem = stem
            .strip_prefix("emery_")
            .or_else(|| stem.strip_prefix("emery-"))
            .unwrap_or(stem);

        Ok(Self::Component {
            name: stem.replace('_', "-"),
            path: PathBuf::from(path),
        })
    }

    // The `omnia:plugins/loader` request this selector names, or `None`
    // for a bare name, which dispatches a statically declared guest.
    fn request(
        &self, pin: Option<&Digest>, registry: Option<&str>,
    ) -> Result<Option<PluginRef>, Error> {
        let (package, location) = match self {
            Self::Bare(_) => return Ok(None),
            Self::Package {
                namespace,
                name,
                version,
            } => (
                format!("{namespace}:{name}@{version}"),
                Location::Registry(registry.map(ToOwned::to_owned)),
            ),
            // The host loader reads the file fresh — nothing is mirrored, so
            // a deleted file refuses on the next run — and would refuse a
            // missing path itself. The engine keeps only the operator-typo
            // gate so a mistyped path lands on `not_found`, the class docs
            // and skills branch on, rather than the loader's `refused`.
            Self::Component { name, path } => {
                let relative = preopen_path(path)?;
                if !relative.is_file() || relative.extension().is_none_or(|ext| ext != "wasm") {
                    let path = path.display();
                    let relative = relative.display();

                    return Err(not_found!(
                        "adapter `{path}` did not resolve to a `.wasm` component at {relative}"
                    ));
                }
                (format!("source:{name}"), Location::Path(relative.display().to_string()))
            }
        };

        Ok(Some(
            PluginRef::builder()
                .package(package)
                .location(location)
                .maybe_digest(pin.cloned())
                .build(),
        ))
    }
}

// Equivalent to `^[a-z][a-z0-9-]*$`.
fn is_kebab(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
