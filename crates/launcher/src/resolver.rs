//! The Emery [`omnia::GuestResolver`]: fail-closed, adapters-only
//! guest resolution over the invocation's one captured
//! [`ExecutionPaths`]. Every settled identity is logged to stderr.

use engine::handler::ExecutionPaths;
use engine::resolve::{AdapterSelector, FIRST_PARTY_NAMESPACE, RoutedId, resolver as locate};
use error::Error;

/// First-party components embedded in the binary as default registry
/// entries (ADR-0002 §2). Empty until Phase 4 ports the first-party
/// adapters; the journey's local-component admission is the permanent
/// out-of-binary conformance rung until then.
const EMBEDDED: &[(&str, &[u8])] = &[];

/// The Emery guest resolver over one captured [`ExecutionPaths`].
///
/// Local-only resolution: project cache seed, embedded first-party
/// registry, verified store entry. There is no download path
/// (ADR-0002 deletions): nothing local is a typed miss.
#[derive(Clone, Debug)]
pub struct Resolver {
    paths: ExecutionPaths,
}

impl Resolver {
    /// Bind the resolver to the invocation's captured layout.
    #[must_use]
    pub const fn new(paths: ExecutionPaths) -> Self {
        Self { paths }
    }

    /// Resolve one adapter identity to its verified component bytes.
    ///
    /// # Errors
    ///
    /// `adapter-not-found` when nothing local satisfies the identity;
    /// `adapter-sidecar-missing` / `adapter-digest-mismatch` /
    /// `adapter-store-unreadable` when a store entry fails
    /// verify-on-read; `adapter-routed-id-malformed` for identities
    /// outside the routed grammar.
    pub fn resolve_component(&self, id: &str) -> Result<Vec<u8>, Error> {
        let routed = RoutedId::parse(id)?;
        // The co-dev seed always wins, pins included — a locally
        // built component would otherwise be shadowed at dispatch.
        if let Some(bytes) = self.seed(&routed)? {
            return Ok(bytes);
        }
        if let Some(bytes) = embedded(&routed.name) {
            log_use(&routed, routed.version.as_ref(), "embedded");
            return Ok(bytes);
        }
        let selector = match routed.version.clone() {
            Some(version) => AdapterSelector::Package {
                namespace: FIRST_PARTY_NAMESPACE.to_string(),
                name: routed.name.clone(),
                version,
            },
            None => AdapterSelector::Bare {
                name: routed.name.clone(),
            },
        };
        let location = locate::locate(routed.axis, &selector, &routed.name, &self.paths)?;
        log_use(&routed, routed.version.as_ref(), "store");
        Ok(std::fs::read(location.path())?)
    }

    /// The seeded project-cache entry for this identity's name, when
    /// one exists. The seed answers pinned and bare identities alike.
    fn seed(&self, routed: &RoutedId) -> Result<Option<Vec<u8>>, Error> {
        let name = routed.name.as_str();
        let bare = AdapterSelector::Bare {
            name: name.to_string(),
        };
        let Ok(location) = locate::locate(routed.axis, &bare, name, &self.paths) else {
            return Ok(None);
        };
        log_use(routed, None, "cache seed");
        Ok(Some(std::fs::read(location.path())?))
    }
}

/// The embedded first-party component for `name`, when the binary
/// carries one.
fn embedded(name: &str) -> Option<Vec<u8>> {
    EMBEDDED
        .iter()
        .find_map(|(entry, bytes)| (*entry == name).then(|| bytes.to_vec()))
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
            let bytes = resolver.resolve_component(guest.as_str()).map_err(anyhow::Error::new)?;
            Ok(Some(omnia::GuestArtifact::wasm(bytes)))
        })
    }
}
