//! The `Source` capability
//!
//! [`Source`] is how the engine reaches an adapter: it addresses a loaded
//! adapter by id and asks it to extract evidence or report its metadata.
//! It follows the shape of omnia's other capability traits so a provider
//! carries it alongside `Model`, storage, and plugin loading.
//!
//! In a wasm guest the trait dispatches over the WIT import automatically. In
//! a native build the methods are left for the caller to implement, so a test
//! can script exactly what an adapter would have returned.

use std::future::Future;

use omnia_guest::Error;

use crate::types::{Evidence, SourceInput, SourceMetadata};

/// Import-side source dispatch over the `emery:adapter/source` contract.
///
/// Adapters implement the export-side `SourceAdapter` from `emery-adapter`
/// instead. An extract failure arrives classified: an adapter refusing its
/// input is `BadRequest`, any other failure `BadGateway`.
pub trait Source: Send + Sync {
    /// Dispatches `extract` to `id`.
    #[cfg(not(target_arch = "wasm32"))]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;

    /// Dispatches `extract` to `id`.
    #[cfg(target_arch = "wasm32")]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        crate::wire::import::extract(id, input)
    }

    /// Returns resolve-time metadata for `id`.
    #[cfg(not(target_arch = "wasm32"))]
    fn metadata(&self, id: &str) -> SourceMetadata;

    /// Returns resolve-time metadata for `id`.
    #[cfg(target_arch = "wasm32")]
    fn metadata(&self, id: &str) -> SourceMetadata {
        crate::wire::import::metadata(id)
    }
}
