//! Stage-and-install of a freshly downloaded tool into the global
//! cache. The byte download (`https:` or `file:`) itself lives in
//! `crate::resolver`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{fs, io};

use tempfile::Builder;

use super::sorted_dir_entries;
use crate::error::ExtensionError;

/// Install a staged cache directory into `dest`.
///
/// The staged tree is first copied into a sibling temporary directory. The
/// final switch into place uses `rename`. When replacing an existing cache
/// version, the old directory is first renamed to a sibling backup, then the
/// new complete directory is renamed into place. A crash during replacement
/// can leave the destination absent plus a backup, but never a partially
/// copied destination.
///
/// # Errors
///
/// Returns the `tool-io` diagnostic when `staged` is not a directory, an
/// entry inside it is neither a file nor a directory, the parent of
/// `dest` cannot be created, a unique sibling temporary path cannot be
/// allocated, or any individual file copy fails. Returns the
/// `tool-cache-root` diagnostic when `dest` has no parent component, and
/// the `tool-atomic-move-failed` diagnostic when the rename of the
/// existing destination to a sibling backup or the rename of the new
/// tree into place fails (in the latter case the previous tree is
/// restored on a best-effort basis and the in-progress copy is removed).
pub fn stage_and_install(staged: &Path, dest: &Path) -> Result<(), ExtensionError> {
    if !staged.is_dir() {
        return Err(ExtensionError::cache_io(
            "inspect staged directory",
            staged,
            io::Error::new(io::ErrorKind::InvalidInput, "staged path is not a directory"),
        ));
    }
    let Some(parent) = dest.parent() else {
        return Err(ExtensionError::cache_root(format!(
            "destination path has no parent: {}",
            dest.display()
        )));
    };
    fs::create_dir_all(parent)
        .map_err(|err| ExtensionError::cache_io("create cache parent", parent, err))?;

    let install_dir =
        unique_sibling_dir(parent, dest.file_name().unwrap_or_else(|| OsStr::new("tool")))?;
    copy_dir_contents(staged, &install_dir)?;

    let backup = if dest.exists() {
        let backup = unique_sibling_backup(parent)?;
        fs::rename(dest, &backup).map_err(|err| {
            ExtensionError::atomic_move_failed(dest.to_path_buf(), backup.clone(), err)
        })?;
        Some(backup)
    } else {
        None
    };

    match fs::rename(&install_dir, dest) {
        Ok(()) => {
            if let Some(backup) = backup {
                fs::remove_dir_all(&backup).map_err(|err| {
                    ExtensionError::cache_io("remove previous cache directory", backup, err)
                })?;
            }
            Ok(())
        }
        Err(source) => {
            if let Some(backup) = &backup {
                drop(fs::rename(backup, dest));
            }
            drop(fs::remove_dir_all(&install_dir));
            Err(ExtensionError::atomic_move_failed(install_dir, dest.to_path_buf(), source))
        }
    }
}

fn unique_sibling_dir(parent: &Path, stem: impl AsRef<OsStr>) -> Result<PathBuf, ExtensionError> {
    let stem = stem.as_ref().to_string_lossy();
    let prefix = format!(".{stem}.");
    let temp = Builder::new()
        .prefix(&prefix)
        .suffix(".tmp")
        .tempdir_in(parent)
        .map_err(|err| ExtensionError::cache_io("create cache temp directory", parent, err))?;
    Ok(temp.keep())
}

fn unique_sibling_backup(parent: &Path) -> Result<PathBuf, ExtensionError> {
    let temp =
        Builder::new().prefix(".previous.").suffix(".tmp").tempdir_in(parent).map_err(|err| {
            ExtensionError::cache_io("create cache backup directory", parent, err)
        })?;
    let backup = temp.keep();
    // Remove the empty placeholder so the subsequent `fs::rename(dest, backup)` succeeds.
    fs::remove_dir(&backup)
        .map_err(|err| ExtensionError::cache_io("clear cache backup placeholder", &backup, err))?;
    Ok(backup)
}

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), ExtensionError> {
    for entry in sorted_dir_entries(src)? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|err| ExtensionError::cache_io("inspect staged entry", &src_path, err))?;
        if file_type.is_dir() {
            fs::create_dir_all(&dst_path).map_err(|err| {
                ExtensionError::cache_io("create staged subdirectory", &dst_path, err)
            })?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)
                .map_err(|err| ExtensionError::cache_io("copy staged file", &src_path, err))?;
        } else {
            return Err(ExtensionError::cache_io(
                "copy staged entry",
                &src_path,
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "staged entries must be files or directories",
                ),
            ));
        }
    }
    Ok(())
}
