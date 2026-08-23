//! Storage-location formulas.

/// Blobstore container of the project component cache.
pub const ADAPTERS_CONTAINER: &str = "adapters";

/// Well-known adapter storage locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Locations;

impl Locations {
    /// Returns a mirrored component object name.
    #[must_use]
    pub fn component_object(&self, name: &str) -> String {
        format!("{name}.wasm")
    }
}
