//! Error shaping over the storage capabilities.
//!
//! The engine reaches its state only through the deployment's
//! [`omnia_guest::StateStore`] / [`omnia_guest::BlobStore`] providers —
//! `wasi:keyvalue` / `wasi:blobstore` imports on wasm32, scripted
//! stores in the native suites (design/portable-storage.md step 2).
//! This module owns the one typed mapping of a capability failure.

// Map a storage capability failure onto the typed engine error: the
// stable `storage-failed` discriminant plus the acting context.
pub fn failed(action: &str, err: &anyhow::Error) -> emery_error::Error {
    emery_error::Error::Diag {
        code: "storage-failed",
        detail: format!("{action}: {err:#}"),
    }
}
