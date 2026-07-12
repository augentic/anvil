//! SHA-256 digest helpers shared across cache, fingerprint, and tool paths.
//!
//! Lives on the `schema` leaf so sibling crates (`workflow`, …) and the
//! in-crate [`crate::diagnostics`] fingerprint share one digest
//! implementation without each depending on `sha2` directly.

use sha2::{Digest, Sha256};

/// Lowercase hex encoding of a SHA-256 digest over `bytes`.
///
/// ```
/// use schema::digest::sha256_hex;
///
/// assert_eq!(sha256_hex(b"").len(), 64);
/// assert!(sha256_hex(b"specify").starts_with(|c: char| c.is_ascii_hexdigit()));
/// ```
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    sha256_output_hex(Sha256::digest(bytes))
}

fn sha256_output_hex(digest: impl AsRef<[u8]>) -> String {
    base16ct::lower::encode_string(digest.as_ref())
}

/// Incremental SHA-256 hasher for streamed input.
///
/// Wraps [`sha2::Sha256`] so callers that hash chunk-by-chunk (download
/// streams, large file reads) do not depend on `sha2` directly — this
/// module is the single home for the digest dependency.
///
/// ```
/// use schema::digest::{Hasher, sha256_hex};
///
/// let mut hasher = Hasher::new();
/// hasher.update(b"spec");
/// hasher.update(b"ify");
/// assert_eq!(hasher.finalize_hex(), sha256_hex(b"specify"));
/// ```
#[derive(Default)]
pub struct Hasher(Sha256);

impl std::fmt::Debug for Hasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hasher").finish_non_exhaustive()
    }
}

impl Hasher {
    /// Create an empty hasher.
    #[must_use]
    pub fn new() -> Self {
        Self(Sha256::new())
    }

    /// Fold `chunk` into the running digest.
    pub fn update(&mut self, chunk: &[u8]) {
        self.0.update(chunk);
    }

    /// Consume the hasher and return the lowercase hex digest.
    #[must_use]
    pub fn finalize_hex(self) -> String {
        sha256_output_hex(self.0.finalize())
    }
}
