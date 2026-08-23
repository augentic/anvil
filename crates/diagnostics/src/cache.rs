//! Content digests for cache identity.

use crate::digest::Hasher;

/// Return an in-memory payload's `sha256:<hex>` digest.
#[must_use]
pub fn content_digest(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    format!("sha256:{}", hasher.finalize_hex())
}
