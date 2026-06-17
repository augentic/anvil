//! `archive` verb: move a slice directory under `<archive>/YYYY-MM-DD-<name>/`.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use specify_error::Error;

use super::io::move_atomic;

/// Move `slice_dir` to `<archive_dir>/YYYY-MM-DD-<slice-name>/`.
///
/// This is the sole implementation of the archive move semantics; both
/// `specify slice archive` and the `specify slice merge run` success path
/// route through it. Does **not** touch `metadata.yaml` — the caller is
/// responsible for any status transition before or after.
///
/// # Errors
///
/// `Error::Diag` with `slice-dir-no-basename` if `slice_dir` lacks a
/// basename; otherwise propagates I/O failures from `create_dir_all`
/// or the rename.
pub fn archive(slice_dir: &Path, archive_dir: &Path, today: Timestamp) -> Result<PathBuf, Error> {
    let slice_name = slice_dir.file_name().and_then(|s| s.to_str()).ok_or_else(|| Error::Diag {
        code: "slice-dir-no-basename",
        detail: format!("slice dir {} has no basename", slice_dir.display()),
    })?;
    let date = today.strftime("%Y-%m-%d").to_string();
    let target = archive_dir.join(format!("{date}-{slice_name}"));
    std::fs::create_dir_all(archive_dir)?;
    move_atomic(slice_dir, &target)?;
    Ok(target)
}

#[cfg(test)]
mod tests;
