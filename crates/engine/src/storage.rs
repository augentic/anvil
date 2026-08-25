//! Storage-capability error shaping.

use omnia_guest::{Error, server_error};

/// Maps a storage failure to a `ServerError`.
pub fn failed(action: &str, err: &anyhow::Error) -> Error {
    server_error!("storage-failed: {action}: {err:#}",)
}
