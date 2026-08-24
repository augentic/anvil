//! Transport-neutral command plumbing: the operation error alias,
//! text-mode rendering, and preopen-relative path normalization.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

/// Operation error type: Omnia's protocol error.
pub type Error = omnia_guest::Error;

/// A `BadRequest` with a kebab `code`.
pub fn bad_request(code: &'static str, description: impl Into<String>) -> Error {
    Error::BadRequest {
        code: code.to_string(),
        description: description.into(),
    }
}

/// A `NotFound` with a kebab `code`.
pub fn not_found(code: &'static str, description: impl Into<String>) -> Error {
    Error::NotFound {
        code: code.to_string(),
        description: description.into(),
    }
}

/// A `ServerError` with a kebab `code`.
pub fn server_error(code: &'static str, description: impl Into<String>) -> Error {
    Error::ServerError {
        code: code.to_string(),
        description: description.into(),
    }
}

/// A `BadGateway` with a kebab `code`.
pub fn bad_gateway(code: &'static str, description: impl Into<String>) -> Error {
    Error::BadGateway {
        code: code.to_string(),
        description: description.into(),
    }
}

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
    bad_request(
        "argument",
        format!(
            "invalid argument {flag}: path `{}` must be relative to the project preopen `.` and \
             must not escape it",
            path.display()
        ),
    )
}
