//! Preopen-relative path normalization: every operator path anchors
//! inside the `.` project mount.

use std::path::{Component, Path, PathBuf};

use omnia_guest::{Error, bad_request};

/// Normalizes an operator path inside the `.` project preopen.
///
/// # Errors
///
/// Returns a `BadRequest` for an absolute path or a relative path that
/// escapes above the project root.
pub fn preopen_path(path: &Path) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        return Err(outside_project(path));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir if normalized.pop() => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(outside_project(path));
            }
        }
    }
    Ok(if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized })
}

fn outside_project(path: &Path) -> Error {
    let path = path.display();
    bad_request!("path `{path}` must be relative to the project root `.` and must not escape it")
}
