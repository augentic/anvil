//! Source-capability dispatch, shaped like [`omnia_guest::Model`].
//!
//! Wasm defaults call WIT imports; native signatures support scripted tests.

use std::future::Future;

use crate::types::{Evidence, SourceInput, SourceMetadata};

/// Source import dispatch failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DispatchError {
    /// Adapter call failure.
    #[error(transparent)]
    Call(#[from] crate::types::Error),
    /// Non-canonical JSON in an open extra (A8).
    #[error("extra `{key}` is not canonical JSON ({detail}): {encoded}")]
    Extras {
        /// Extra key.
        key: String,
        /// Parse failure.
        detail: String,
        /// Wire value.
        encoded: String,
    },
}

/// Import-side source dispatch over the `emery:adapter/source` contract.
///
/// Adapters implement the export-side [`crate::SourceAdapter`] instead.
pub trait Source: Send + Sync {
    /// Dispatches `extract` to `id`.
    #[cfg(not(target_arch = "wasm32"))]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send;

    /// Dispatches `extract` to `id`.
    #[cfg(target_arch = "wasm32")]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send {
        crate::source::import::extract(id, input)
    }

    /// Returns resolve-time metadata for `id`.
    #[cfg(not(target_arch = "wasm32"))]
    fn metadata(&self, id: &str) -> SourceMetadata;

    /// Returns resolve-time metadata for `id`.
    #[cfg(target_arch = "wasm32")]
    fn metadata(&self, id: &str) -> SourceMetadata {
        crate::source::import::metadata(id)
    }
}
