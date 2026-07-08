//! Cross-device-safe `move_atomic` helper shared by archive / discard
//! verbs and `crate::change::plan` plan-archive moves.

use std::io;
use std::path::Path;

use error::Error;

/// Move `src` to `dst`, falling back to copy-then-remove across mounts.
///
/// Uses `rename` first, then falls back on
/// [`io::ErrorKind::CrossesDevices`] (`EXDEV` / `ERROR_NOT_SAME_DEVICE`
/// — std maps the platform code) so archives on a different mount from
/// the working tree still work.
///
/// Dispatches on `src.is_dir()`: directories copy recursively, files
/// via a single `std::fs::copy`. The two old helpers
/// (`move_file_atomic`, `move_dir_atomic`) were identical modulo that
/// one branch — collapsing them keeps the cross-device semantics in a
/// single implementation shared by `crate::merge::slice` (archive
/// move) and `crate::change::plan` (plan archive move).
///
/// # Errors
///
/// Returns `Error::Io` on rename / copy / remove failures.
pub fn move_atomic(src: &Path, dst: &Path) -> Result<(), Error> {
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::CrossesDevices => {
            if src.is_dir() {
                copy_dir_recursive(src, dst)?;
                std::fs::remove_dir_all(src)?;
            } else {
                std::fs::copy(src, dst)?;
                std::fs::remove_file(src)?;
            }
            Ok(())
        }
        Err(err) => Err(Error::Io(err)),
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if file_type.is_symlink() {
            let link_target = std::fs::read_link(entry.path())?;
            symlink(&link_target, &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn symlink(original: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

#[cfg(windows)]
fn symlink(original: &Path, link: &Path) -> io::Result<()> {
    match std::fs::metadata(original) {
        Ok(meta) if meta.is_dir() => std::os::windows::fs::symlink_dir(original, link),
        _ => std::os::windows::fs::symlink_file(original, link),
    }
}

#[cfg(not(any(unix, windows)))]
fn symlink(_original: &Path, _link: &Path) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "symlinks unsupported on this platform"))
}
