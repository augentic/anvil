//! Storage-capability error shaping.

use crate::handler::{Error, server_error};

/// Maps a storage failure to the stable `storage-failed` error.
pub fn failed(action: &str, err: &anyhow::Error) -> Error {
    server_error("storage-failed", format!("storage-failed: {action}: {err:#}"))
}
