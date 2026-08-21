//! The source-seam dispatch capability, shaped like
//! [`omnia_guest::Model`]: default `wasm32` bodies delegate to the WIT
//! imports; off `wasm32` the bare signatures let tests script the seam.

use std::future::Future;

use crate::seam::{Evidence, SourceInput, SourceMetadata};

/// Import dispatch failure: the operation's seam error, or an
/// evidence extra whose wire value is not canonical JSON.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DispatchError {
    /// The dispatched operation failed across the seam.
    #[error(transparent)]
    Seam(#[from] crate::seam::Error),
    /// An open extra failed the canonical JSON parse (A8).
    #[error("extra `{key}` is not canonical JSON ({detail}): {encoded}")]
    Extras {
        /// The extra's key.
        key: String,
        /// The parse failure.
        detail: String,
        /// The value as it crossed the wire.
        encoded: String,
    },
}

/// Guest-to-guest dispatch over the `emery:adapter/source` seam.
///
/// The engine provider's import-side capability, shaped like
/// [`omnia_guest::Model`]. Distinct from [`crate::SourceAdapter`],
/// which adapters implement on the export side of the same WIT world.
pub trait Source: Send + Sync {
    /// Dispatch `extract` on the source component routed by `id`.
    #[cfg(not(target_arch = "wasm32"))]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send;

    /// Dispatch `extract` on the source component routed by `id`.
    #[cfg(target_arch = "wasm32")]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send {
        crate::source::import::extract(id, input)
    }

    /// Resolve-time metadata of the source component routed by `id`.
    #[cfg(not(target_arch = "wasm32"))]
    fn metadata(&self, id: &str) -> SourceMetadata;

    /// Resolve-time metadata of the source component routed by `id`.
    #[cfg(target_arch = "wasm32")]
    fn metadata(&self, id: &str) -> SourceMetadata {
        crate::source::import::metadata(id)
    }
}
