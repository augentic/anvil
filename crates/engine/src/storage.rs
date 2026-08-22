//! Storage-capability error shaping.

/// Maps a storage failure to the stable `storage-failed` error.
pub fn failed(action: &str, err: &anyhow::Error) -> emery_error::Error {
    emery_error::Error::Diag {
        code: "storage-failed",
        detail: format!("{action}: {err:#}"),
    }
}
