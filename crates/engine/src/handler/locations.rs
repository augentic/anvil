//! [`Locations`] — the engine's storage layout as fixed key and
//! container-name formulas over the deployment's storage capabilities.
//! Kernels never read `std::env` themselves.

/// Blobstore container of the project component cache.
pub const ADAPTERS_CONTAINER: &str = "adapters";

/// Blobstore container of the global adapter store.
pub const STORE_CONTAINER: &str = "store";

/// Well-known storage locations for resolvable artifacts.
///
/// Owns the naming formulas over the two fixed containers — the
/// global adapter store and the project component cache. Methods are
/// pure key/object-name math over the container constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Locations;

impl Locations {
    /// Object name of the mirrored component inside
    /// [`ADAPTERS_CONTAINER`].
    #[must_use]
    pub fn component_object(&self, name: &str) -> String {
        format!("{name}.wasm")
    }

    /// Object name of an immutable `(name, version)` store entry
    /// inside [`STORE_CONTAINER`].
    ///
    /// The store is keyed by the pinned identity, not the project, so
    /// two projects pinning the same `(name, version)` resolve to one
    /// shared, immutable entry (the Cargo `~/.cargo/registry` model).
    #[must_use]
    pub fn store_object(&self, name: &str, version: &str) -> String {
        format!("{name}@{version}.wasm")
    }

    /// Keyvalue key of the verify-on-read digest sidecar for a
    /// [`Self::store_object`] entry.
    ///
    /// A keyvalue *sibling* of the blob, never the blob itself: the
    /// sidecar is a writable provenance record that must not perturb
    /// the read-only immutability of the installed component bytes.
    #[must_use]
    pub fn store_meta_key(&self, name: &str, version: &str) -> String {
        format!("store/{name}@{version}.meta")
    }
}
