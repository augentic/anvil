//! Storage-location formulas.

/// Blobstore container of the project component cache.
pub const ADAPTERS_CONTAINER: &str = "adapters";

/// Blobstore container of the global adapter store.
pub const STORE_CONTAINER: &str = "store";

/// Well-known adapter storage locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Locations;

impl Locations {
    /// Returns a mirrored component object name.
    #[must_use]
    pub fn component_object(&self, name: &str) -> String {
        format!("{name}.wasm")
    }

    /// Returns an immutable store object name.
    #[must_use]
    pub fn store_object(&self, name: &str, version: &str) -> String {
        format!("{name}@{version}.wasm")
    }

    /// Returns the digest-sidecar key for a store object.
    ///
    /// Sidecars remain writable keyvalue records beside immutable blobs.
    #[must_use]
    pub fn store_meta_key(&self, name: &str, version: &str) -> String {
        format!("store/{name}@{version}.meta")
    }
}
