//! Storage-capability error shaping.

use emery_error::Error as Legacy;

use crate::handler::{Error, classify};

/// Maps a storage failure to the stable `storage-failed` error.
pub fn failed(action: &str, err: &anyhow::Error) -> Error {
    classify(&Legacy::Diag {
        code: "storage-failed",
        detail: format!("{action}: {err:#}"),
    })
}
