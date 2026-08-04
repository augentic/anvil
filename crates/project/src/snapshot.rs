//! Snapshot identity and code-patch vocabulary (RFC-87).
//!
//! A **snapshot** is the immutable, content-addressed identity of a
//! complete product-code tree; a **code patch** is the immutable
//! relation between two snapshots plus the touched paths derived by
//! comparing them. These are the value-layer nouns shared with the
//! RFC-86 fact substrate: facts and build records reference snapshot
//! identities, never workspace paths. The storage and materialization
//! kernel lives in [`crate::workspace`]; this module is the wasm-safe
//! wire vocabulary only.

use std::fmt;

use error::Error;
use serde::{Deserialize, Serialize};

/// Scheme prefix of the canonical snapshot-id wire form.
const SCHEME: &str = "sha256:";

/// Content-addressed identity of a complete product-code tree.
///
/// Wire form is `sha256:<64 lowercase hex>` — the digest of the
/// snapshot's canonical tree manifest. Rides the WIT `revision` type
/// across the seam as an opaque string.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SnapshotId(String);

impl SnapshotId {
    /// Wrap a lowercase-hex SHA-256 digest as a snapshot identity.
    #[must_use]
    pub fn from_digest(hex: &str) -> Self {
        Self(format!("{SCHEME}{hex}"))
    }

    /// Parse the canonical `sha256:<64 lowercase hex>` wire form.
    ///
    /// # Errors
    ///
    /// `snapshot-id-malformed` when the scheme or digest shape is wrong.
    pub fn parse(value: &str) -> Result<Self, Error> {
        let malformed = || Error::Diag {
            code: "snapshot-id-malformed",
            detail: format!("snapshot id `{value}` is not `sha256:<64 lowercase hex>`"),
        };
        let hex = value.strip_prefix(SCHEME).ok_or_else(malformed)?;
        if hex.len() != 64 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(malformed());
        }
        Ok(Self(value.to_string()))
    }

    /// The canonical wire form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The bare lowercase-hex digest without the scheme prefix.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.0[SCHEME.len()..]
    }
}

impl fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for SnapshotId {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<SnapshotId> for String {
    fn from(id: SnapshotId) -> Self {
        id.0
    }
}

/// The immutable relation between a base and result snapshot.
///
/// `touched` carries the workspace-relative, `/`-separated paths that
/// differ between the two trees, sorted — the authoritative record of
/// what an execution changed. There is no separately encoded patch
/// blob: binary content, deletes, modes, and symlinks are properties
/// of the two trees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CodePatch {
    /// The snapshot the execution started from.
    pub base: SnapshotId,
    /// The snapshot captured from the execution's result tree.
    pub result: SnapshotId,
    /// Sorted workspace-relative paths that differ between the trees.
    pub touched: Vec<String>,
}
