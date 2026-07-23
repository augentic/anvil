//! The Specify [`omnia::GuestResolver`]: fail-closed, adapters-only
//! guest resolution over the invocation's one captured
//! [`ExecutionPaths`].
//!
//! The engine guest is embedded in the binary and registered
//! statically at boot, so no engine identity ever reaches this
//! resolver. An unpinned routed id (`<axis>:<name>`) maps to the
//! anchored project's seeded component cache and is verify-and-load
//! only — its miss is the hard, deterministic `adapter-not-found`. A
//! pinned routed id (`<axis>:<name>@<version>`) maps to the immutable
//! global store entry; a store miss triggers the host-owned
//! pull-on-miss install from the first-party OCI registry
//! ([`crate::install`]), then resolves the freshly installed entry
//! through the same verify-on-read gate. Malformed identities fail
//! deterministically.

use error::Error;
use project::adapter::{AdapterSelector, FIRST_PARTY_NAMESPACE, RoutedId, resolver as locate};
use project::handler::ExecutionPaths;

use crate::install::{self, Registry};

/// The Specify guest resolver: adapter resolution (with the
/// package-pin pull-on-miss install leg) over one captured
/// [`ExecutionPaths`].
#[derive(Clone, Debug)]
pub struct Resolver {
    paths: ExecutionPaths,
    registry: Registry,
}

impl Resolver {
    /// Bind the resolver to the invocation's captured layout, over
    /// the compiled first-party registry.
    #[must_use]
    pub(crate) fn new(paths: ExecutionPaths) -> Self {
        Self::with_registry(paths, Registry::first_party())
    }

    /// Bind the resolver to an explicit registry base — the
    /// integration-test seam. The shipped composition always goes
    /// through [`crate::Policy::resolver`], which hard-codes the
    /// first-party registry.
    #[must_use]
    pub const fn with_registry(paths: ExecutionPaths, registry: Registry) -> Self {
        Self { paths, registry }
    }

    /// Resolve one adapter identity to its verified component bytes,
    /// installing a missing package pin from the registry first.
    ///
    /// # Errors
    ///
    /// `adapter-not-found` when an unpinned identity has no cached
    /// component; `adapter-install-failed` / `adapter-install-invalid`
    /// when a cold pinned miss cannot be installed (offline, missing
    /// tag, malformed artifact); `adapter-sidecar-missing` /
    /// `adapter-digest-mismatch` when a store entry is unverifiable;
    /// `adapter-routed-id-malformed` for identities outside the
    /// routed grammar.
    pub async fn resolve_component(&self, id: &str) -> Result<Vec<u8>, Error> {
        let routed = RoutedId::parse(id)?;
        let selector = match &routed.version {
            Some(version) => {
                // Pull-on-miss: a pinned identity whose store entry is
                // absent is installed from the first-party registry
                // before the verify-on-read resolve below.
                let version_str = version.to_string();
                if !self.paths.locations().store_entry(&routed.name, &version_str).is_file() {
                    install::install(&self.registry, &routed.name, &version_str, &self.paths)
                        .await?;
                }
                AdapterSelector::Package {
                    namespace: FIRST_PARTY_NAMESPACE.to_string(),
                    name: routed.name.clone(),
                    version: version.clone(),
                }
            }
            None => AdapterSelector::Bare {
                name: routed.name.clone(),
            },
        };
        // `locate` probes the immutable store entry (fail-closed
        // verify-on-read) for a pin, else the anchored project's
        // component cache — and its miss is the canonical
        // `adapter-not-found`.
        let location = locate::locate(routed.axis, &selector, &routed.name, &self.paths)?;
        Ok(std::fs::read(location.path())?)
    }
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
