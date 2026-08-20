//! SHA-256 digest helpers shared across cache, fingerprint, and tool paths.
//!
//! One digest implementation, so consumers never depend on `sha2` directly.

use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

// Stream buffer for [`Hasher::update_reader`]: large enough to keep
// syscall count down, small enough for the stack.
const STREAM_CHUNK: usize = 16 * 1024;

/// Lowercase hex encoding of a SHA-256 digest over `bytes`.
///
/// ```
/// use emery_diagnostics::digest::sha256_hex;
///
/// assert_eq!(sha256_hex(b"").len(), 64);
/// assert!(sha256_hex(b"emery").starts_with(|c: char| c.is_ascii_hexdigit()));
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
/// use emery_diagnostics::digest::{Hasher, sha256_hex};
///
/// let mut hasher = Hasher::new();
/// hasher.update(b"em");
/// hasher.update(b"ery");
/// assert_eq!(hasher.finalize_hex(), sha256_hex(b"emery"));
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

    /// Fold every byte from `reader` into the running digest.
    ///
    /// # Errors
    ///
    /// Returns the reader's I/O error. Bytes consumed before the
    /// failure remain in the hasher.
    pub fn update_reader(&mut self, reader: &mut impl Read) -> io::Result<()> {
        let mut buf = [0_u8; STREAM_CHUNK];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                return Ok(());
            }
            self.update(&buf[..n]);
        }
    }

    /// Consume the hasher and return the lowercase hex digest.
    #[must_use]
    pub fn finalize_hex(self) -> String {
        sha256_output_hex(self.0.finalize())
    }
}

/// SHA-256 of every byte from `reader`, streamed.
///
/// ```
/// use std::io::Cursor;
///
/// use emery_diagnostics::digest::{sha256_hex, sha256_reader};
///
/// let mut cursor = Cursor::new(b"emery");
/// assert_eq!(sha256_reader(&mut cursor).unwrap(), sha256_hex(b"emery"));
/// ```
///
/// # Errors
///
/// Returns the reader's I/O error.
pub fn sha256_reader(reader: &mut impl Read) -> io::Result<String> {
    let mut hasher = Hasher::new();
    hasher.update_reader(reader)?;
    Ok(hasher.finalize_hex())
}

/// SHA-256 of the file at `path`, streamed so the contents need not
/// fit in memory.
///
/// # Errors
///
/// Returns an I/O error when the file cannot be opened or read.
pub fn sha256_path(path: &Path) -> io::Result<String> {
    sha256_reader(&mut std::fs::File::open(path)?)
}
