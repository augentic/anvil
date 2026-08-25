//! Storage-capability error shaping.

use omnia_guest::Error;

/// Maps a storage failure to the stable `storage-failed` error.
pub fn failed(action: &str, err: &anyhow::Error) -> Error {
    Error::ServerError {
        code: "storage-failed".into(),
        description: format!("storage-failed: {action}: {err:#}"),
    }
}
