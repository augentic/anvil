//! Shared SHA-256 helpers.

use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

const STREAM_CHUNK: usize = 16 * 1024;

/// Lowercase hex encoding of a SHA-256 digest over `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    sha256_output_hex(Sha256::digest(bytes))
}

fn sha256_output_hex(digest: impl AsRef<[u8]>) -> String {
    base16ct::lower::encode_string(digest.as_ref())
}

/// Incremental SHA-256 hasher for streamed input.
///
/// Keeps consumers independent of the underlying digest crate.
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
    /// Returns the reader's I/O error; consumed bytes remain hashed.
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
/// # Errors
///
/// Returns the reader's I/O error.
pub fn sha256_reader(reader: &mut impl Read) -> io::Result<String> {
    let mut hasher = Hasher::new();
    hasher.update_reader(reader)?;
    Ok(hasher.finalize_hex())
}

/// Stream the file at `path` into SHA-256.
///
/// # Errors
///
/// Returns an I/O error when the file cannot be opened or read.
pub fn sha256_path(path: &Path) -> io::Result<String> {
    sha256_reader(&mut std::fs::File::open(path)?)
}
