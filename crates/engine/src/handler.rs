//! Transport-neutral command plumbing: the operation error alias,
//! text-mode rendering, and preopen-relative path normalization.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

/// Operation error type: the workspace taxonomy.
pub type Error = emery_error::Error;

/// Maps a workspace error onto the Omnia protocol class and kebab code.
///
/// The description is [`Error`]'s display so text-mode stderr stays the same.
pub fn classify(err: &Error) -> omnia_guest::Error {
    let description = err.to_string();
    match err {
        Error::Argument { .. } | Error::Validation { .. } | Error::AdapterCliTooOld { .. } => {
            omnia_guest::Error::BadRequest {
                code: err.variant_str().into_owned(),
                description,
            }
        }
        Error::Filesystem { .. } | Error::Io(_) => omnia_guest::Error::ServerError {
            code: err.variant_str().into_owned(),
            description,
        },
        Error::Diag { code, .. } => classify_diag(code, description),
    }
}

fn classify_diag(code: &'static str, description: String) -> omnia_guest::Error {
    let owned = code.to_string();
    match code {
        "argument"
        | "specify-source-required"
        | "specify-source-duplicate"
        | "claim-invalid"
        | "claim-extras-missing"
        | "spec-invalid"
        | "spec-provenance-mismatch"
        | "design-empty"
        | "adapter-floor-malformed"
        | "adapter-cli-too-old"
        | "adapter-arg-malformed"
        | "adapter-github-uri-unsupported"
        | "adapter-package-ref-version-required"
        | "adapter-package-ref-malformed"
        | "adapter-dir-name-unresolved"
        | "sources-toml-malformed"
        | "source-remote-unsupported" => omnia_guest::Error::BadRequest {
            code: owned,
            description,
        },
        "spec-not-generated" | "adapter-component-missing" => omnia_guest::Error::NotFound {
            code: owned,
            description,
        },
        "source-extract-failed" | "claim-extras-malformed" | "synthesis-model-failed" => {
            omnia_guest::Error::BadGateway {
                code: owned,
                description,
            }
        }
        _ => omnia_guest::Error::ServerError {
            code: owned,
            description,
        },
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
