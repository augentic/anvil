//! The Specify [`omnia::GuestResolver`]: fail-closed, adapters-only
//! guest resolution over the invocation's one captured
//! [`ExecutionPaths`].
//!
//! Verify-and-load, never download. The engine guest is embedded in
//! the binary and registered statically at boot, so no identity is
//! ever hydrated here. A pinned routed id (`<axis>:<name>@<version>`)
//! maps to the immutable global store entry, an unpinned id
//! (`<axis>:<name>`) to the anchored project's seeded component cache
//! — both populated exclusively by the engine guest's own
//! `ensure_source` / `ensure_target` legs through the writable
//! store/cache mounts *before* any dispatch can miss. A miss here is
//! therefore a hard, deterministic failure — the fail-closed
//! backstop, never a second download path. Malformed identities fail
//! the same way.

use error::Error;
use project::adapter::{AdapterSelector, FIRST_PARTY_NAMESPACE, RoutedId, resolver as locate};
use project::handler::ExecutionPaths;

/// The Specify guest resolver: adapter verify-and-load over one
/// captured [`ExecutionPaths`].
#[derive(Clone, Debug)]
pub struct Resolver {
    paths: ExecutionPaths,
}

impl Resolver {
    /// Bind the resolver to the invocation's captured layout.
    #[must_use]
    pub(crate) const fn new(paths: ExecutionPaths) -> Self {
        Self { paths }
    }

    /// Resolve one adapter identity to its verified component bytes.
    ///
    /// # Errors
    ///
    /// `adapter-not-found` when an identity has no store/cache
    /// component (the fail-closed backstop — adapters are hydrated by
    /// the engine guest's ensure legs, never here);
    /// `adapter-sidecar-missing` / `adapter-digest-mismatch` when a
    /// store entry is unverifiable; `adapter-routed-id-malformed` for
    /// identities outside the routed grammar.
    pub fn resolve_component(&self, id: &str) -> Result<Vec<u8>, Error> {
        let routed = RoutedId::parse(id)?;
        let selector = match &routed.version {
            Some(version) => AdapterSelector::Package {
                namespace: FIRST_PARTY_NAMESPACE.to_string(),
                name: routed.name.clone(),
                version: version.clone(),
            },
            None => AdapterSelector::Bare {
                name: routed.name.clone(),
            },
        };
        // Verify-and-load only: `locate` probes the immutable store
        // entry (verify-on-read) for a pin, else the anchored
        // project's component cache — and its miss is the canonical
        // `adapter-not-found`.
        let location = locate::locate(routed.axis, &selector, &routed.name, &self.paths)?;
        if let Some(version) = &routed.version {
            require_sidecar(&self.paths, &routed.name, &version.to_string())?;
        }
        Ok(std::fs::read(location.path())?)
    }
}

impl omnia::GuestResolver for Resolver {
    fn resolve(
        &self, guest: omnia::GuestId, _expected_export: String,
    ) -> omnia::FutureResult<Option<omnia::GuestArtifact>> {
        let resolver = self.clone();
        Box::pin(async move {
            let bytes = resolver.resolve_component(guest.as_str()).map_err(anyhow::Error::new)?;
            Ok(Some(omnia::GuestArtifact::wasm(bytes)))
        })
    }
}

/// Fail closed on a store entry without a digest sidecar.
///
/// Deliberately stricter than the engine's fail-open
/// `verify_store_entry` (which waves legacy sidecar-less entries
/// through): the resolver is the last gate before the runtime executes
/// the bytes, so an unverifiable store install is refused. Non-store
/// resolves are exempt — cache-backed components are not
/// content-addressed store installs.
fn require_sidecar(paths: &ExecutionPaths, name: &str, version: &str) -> Result<(), Error> {
    let meta = paths.locations().store_meta(name, version);
    if diagnostics::cache::read_store_meta(&meta).is_none() {
        return Err(Error::Diag {
            code: "adapter-sidecar-missing",
            detail: format!(
                "store entry {} has no digest sidecar; the resolver refuses unverifiable \
                 components — reinstall `specify:{name}@{version}` to record one",
                paths.locations().store_entry(name, version).display(),
            ),
        });
    }
    Ok(())
}
