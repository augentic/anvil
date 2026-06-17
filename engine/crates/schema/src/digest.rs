//! SHA-256 digest helpers shared across cache, fingerprint, and tool paths.
//!
//! Lives on the `specify-schema` leaf so siblings (`specify-diagnostics`,
//! `specify-standards`, `specify-tool`, …) share one digest implementation
//! without each depending on `sha2` directly or pulling a heavier graph.

use sha2::{Digest, Sha256};

/// Lowercase hex encoding of a SHA-256 digest over `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    sha256_output_hex(Sha256::digest(bytes))
}

/// Lowercase hex encoding of a finalized SHA-256 output.
#[must_use]
pub fn sha256_output_hex(digest: impl AsRef<[u8]>) -> String {
    base16ct::lower::encode_string(digest.as_ref())
}

/// Incremental SHA-256 hasher for streamed input.
///
/// Wraps [`sha2::Sha256`] so callers that hash chunk-by-chunk (download
/// streams, large file reads) do not depend on `sha2` directly — this
/// module is the single home for the digest dependency.
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

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{sha256_hex, sha256_output_hex};

    #[test]
    fn sha256_hex_empty() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_output_hex_matches_digest() {
        let digest = Sha256::digest(b"specify");
        assert_eq!(sha256_output_hex(digest), sha256_hex(b"specify"));
    }
}
