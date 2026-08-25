//! Transport-neutral command plumbing: text-mode rendering and
//! preopen-relative path normalization.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use omnia_guest::Error;
use serde::Serialize;

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
/// Returns `argument` for an absolute path or a relative path that
/// escapes above the project root.
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
    Error::BadRequest {
        code: "argument".into(),
        description: format!(
            "invalid argument {flag}: path `{}` must be relative to the project preopen `.` and \
             must not escape it",
            path.display()
        ),
    }
}
