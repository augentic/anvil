//! The Emery [`omnia::GuestResolver`]: fail-closed, adapters-only
//! guest resolution over the invocation's one captured
//! [`ExecutionPaths`]. Every settled identity is logged to stderr.

use std::collections::BTreeSet;

use error::Error;
use project::adapter::{AdapterSelector, FIRST_PARTY_NAMESPACE, RoutedId, resolver as locate};
use project::handler::ExecutionPaths;

use crate::install::{self, Registry};

/// The Emery guest resolver: local-first adapter resolution (with the
/// pull-on-miss / pull-latest install legs) over one captured
/// [`ExecutionPaths`].
#[derive(Clone, Debug)]
pub struct Resolver {
    paths: ExecutionPaths,
    registry: Registry,
    /// Bare names the invocation explicitly upgrades: the registry
    /// check runs ahead of the store probe for these (the cache seed
    /// still wins).
    refresh: BTreeSet<String>,
}

impl Resolver {
    /// Bind the resolver to the invocation's captured layout, over
    /// the compiled first-party registry.
    #[must_use]
    pub(crate) fn new(paths: ExecutionPaths, refresh: BTreeSet<String>) -> Self {
        Self {
            paths,
            registry: Registry::first_party(),
            refresh,
        }
    }

    /// Bind the resolver to an explicit registry base — the
    /// integration-test seam. The shipped composition always goes
    /// through [`crate::Policy::resolver`], which hard-codes the
    /// first-party registry.
    #[must_use]
    pub const fn with_registry(paths: ExecutionPaths, registry: Registry) -> Self {
        Self {
            paths,
            registry,
            refresh: BTreeSet::new(),
        }
    }

    /// Mark bare names for an explicit registry refresh — the
    /// integration seam mirroring what [`crate::Policy::new`] derives
    /// from argv (`adapter upgrade`, `init`).
    #[must_use]
    pub fn refreshing(mut self, names: impl IntoIterator<Item = String>) -> Self {
        self.refresh.extend(names);
        self
    }

    /// Resolve one adapter identity to its verified component bytes,
    /// installing a missing package from the registry when nothing
    /// local satisfies it.
    ///
    /// # Errors
    ///
    /// `adapter-latest-failed` / `adapter-latest-none` when an
    /// unpinned identity with nothing local (or an explicit refresh)
    /// cannot list the registry or finds no SemVer tag;
    /// `adapter-install-failed` / `adapter-install-invalid` when a
    /// cold miss cannot be installed (offline, missing tag, malformed
    /// artifact); `adapter-sidecar-missing` / `adapter-digest-mismatch`
    /// when a store entry is unverifiable; `adapter-routed-id-malformed`
    /// for identities outside the routed grammar.
    pub async fn resolve_component(&self, id: &str) -> Result<Vec<u8>, Error> {
        let routed = RoutedId::parse(id)?;
        match routed.version.clone() {
            Some(version) => self.resolve_pinned(&routed, version).await,
            None => self.resolve_bare(&routed).await,
        }
    }

    /// Pinned identity: the immutable store entry, installed on miss.
    /// An entry that fails store verification (a torn install, drifted
    /// bytes) is reinstalled in place once before failing closed.
    async fn resolve_pinned(
        &self, routed: &RoutedId, version: semver::Version,
    ) -> Result<Vec<u8>, Error> {
        let version_str = version.to_string();
        if !self.paths.locations().store_entry(&routed.name, &version_str).is_file() {
            install::install(&self.registry, &routed.name, &version_str, &self.paths).await?;
        }
        let selector = package(&routed.name, version.clone());
        let location = match locate::locate(routed.axis, &selector, &routed.name, &self.paths) {
            Err(err) if unverifiable(&err) => {
                self.heal(&routed.name, &version_str, err).await?;
                locate::locate(routed.axis, &selector, &routed.name, &self.paths)?
            }
            other => other?,
        };
        log_use(routed, Some(&version), "store");
        Ok(std::fs::read(location.path())?)
    }

    /// Unpinned identity: cache seed, else newest store version, else
    /// pull-latest; an explicit refresh forces the registry check
    /// ahead of the store probe.
    async fn resolve_bare(&self, routed: &RoutedId) -> Result<Vec<u8>, Error> {
        let name = routed.name.as_str();

        // The co-dev seed always wins — including over an explicit
        // refresh, which is surfaced rather than silently shadowed.
        let bare = AdapterSelector::Bare {
            name: name.to_string(),
        };
        if let Ok(location) = locate::locate(routed.axis, &bare, name, &self.paths) {
            if self.refresh.contains(name) {
                eprintln!(
                    "emery {}: `{name}` resolves the project cache seed, which always wins; \
                     re-run `emery adapter add` with a newer component (or remove the seed) to \
                     update it",
                    env!("CARGO_PKG_VERSION"),
                );
            }
            log_use(routed, None, "cache seed");
            return Ok(std::fs::read(location.path())?);
        }

        if self.refresh.contains(name) {
            let latest = install::resolve_latest(&self.registry, name).await?;
            let newest = install::store_newest(name, &self.paths);
            if newest.as_ref() < Some(&latest) {
                install::install(&self.registry, name, &latest.to_string(), &self.paths).await?;
            }
        }

        if let Some(version) = install::store_newest(name, &self.paths) {
            let selector = package(name, version.clone());
            // An explicit refresh may reinstall an unverifiable
            // equal-version entry; without one the store-only path
            // stays fail-closed.
            let location = match locate::locate(routed.axis, &selector, name, &self.paths) {
                Err(err) if self.refresh.contains(name) && unverifiable(&err) => {
                    self.heal(name, &version.to_string(), err).await?;
                    locate::locate(routed.axis, &selector, name, &self.paths)?
                }
                other => other?,
            };
            log_use(routed, Some(&version), "store");
            return Ok(std::fs::read(location.path())?);
        }

        // Nothing local: provision the newest published version.
        let latest = install::resolve_latest(&self.registry, name).await?;
        install::install(&self.registry, name, &latest.to_string(), &self.paths).await?;
        let selector = package(name, latest.clone());
        let location = locate::locate(routed.axis, &selector, name, &self.paths)?;
        log_use(routed, Some(&latest), "installed from registry");
        Ok(std::fs::read(location.path())?)
    }

    /// Reinstall an unverifiable store entry's pin over the stale
    /// files — the recovery for a torn install or drifted bytes. The
    /// install writes sidecar-then-entry atomically only after a
    /// successful pull, so a failed reinstall (offline, tag gone)
    /// leaves the local artifact in place and the original
    /// verification refusal stands, logged to stderr.
    async fn heal(&self, name: &str, version: &str, refused: Error) -> Result<(), Error> {
        if let Err(err) = install::install(&self.registry, name, version, &self.paths).await {
            eprintln!(
                "emery {}: reinstalling unverifiable `{name}@{version}` failed: {err}",
                env!("CARGO_PKG_VERSION"),
            );
            return Err(refused);
        }
        Ok(())
    }
}

/// Whether a locate failure is a store-verification refusal
/// (`adapter-sidecar-missing` / `adapter-digest-mismatch`) the install
/// leg can heal by reinstalling the pin. Misses and I/O failures are
/// not healable.
fn unverifiable(err: &Error) -> bool {
    matches!(err, Error::Diag { code, .. }
        if *code == "adapter-sidecar-missing" || *code == "adapter-digest-mismatch")
}

/// The first-party package selector for one `name@version` identity.
fn package(name: &str, version: semver::Version) -> AdapterSelector {
    AdapterSelector::Package {
        namespace: FIRST_PARTY_NAMESPACE.to_string(),
        name: name.to_string(),
        version,
    }
}

/// One stderr line per settled adapter identity — with the host
/// version, the run's version audit trail (project files record no
/// adapter versions).
fn log_use(routed: &RoutedId, version: Option<&semver::Version>, origin: &str) {
    let identity = version.map_or_else(
        || format!("{}:{}", routed.axis.prefix(), routed.name),
        |version| format!("{}:{}@{version}", routed.axis.prefix(), routed.name),
    );
    eprintln!("emery {}: using {identity} ({origin})", env!("CARGO_PKG_VERSION"));
}

impl omnia::GuestResolver for Resolver {
    fn resolve(
        &self, guest: omnia::GuestId, _expected_export: String,
    ) -> omnia::FutureResult<Option<omnia::GuestArtifact>> {
        let resolver = self.clone();
        Box::pin(async move {
            let bytes =
                resolver.resolve_component(guest.as_str()).await.map_err(anyhow::Error::new)?;
            Ok(Some(omnia::GuestArtifact::wasm(bytes)))
        })
    }
}
