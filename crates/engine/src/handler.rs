//! Transport-neutral command plumbing: the operation error alias,
//! text-mode rendering, and preopen-relative path normalization.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

/// Operation error type: the workspace taxonomy.
pub type Error = emery_error::Error;

/// Human-readable rendering for a serializable command body.
pub trait Render: Serialize {
    /// Writes `self` to `w`.
    ///
    /// # Errors
    ///
    /// Propagates I/O errors.
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()>;
}

/// Normalizes an operator path inside the `.` project preopen.
///
/// # Errors
///
/// Returns [`Error::Argument`] for an absolute path or a relative path
/// that escapes above the project root.
pub fn preopen_path(path: &Path, argument: &'static str) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        return Err(outside_project(path, argument));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir if normalized.pop() => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(outside_project(path, argument));
            }
        }
    }
    Ok(if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized })
}

fn outside_project(path: &Path, flag: &'static str) -> Error {
    Error::Argument {
        flag,
        detail: format!(
            "path `{}` must be relative to the project preopen `.` and must not escape it",
            path.display()
        ),
    }
}
