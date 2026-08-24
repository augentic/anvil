//! Transport-neutral command plumbing: the operation error alias,
//! text-mode rendering, and preopen-relative path normalization.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use emery_error::Error as Legacy;
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

/// Maps a workspace error onto the Omnia protocol class and kebab code.
///
/// The description is the legacy display so text-mode stderr stays the same.
pub fn classify(err: &Legacy) -> Error {
    let description = err.to_string();
    match err {
        Legacy::Argument { .. } | Legacy::Validation { .. } | Legacy::AdapterCliTooOld { .. } => {
            Error::BadRequest {
                code: err.variant_str().into_owned(),
                description,
            }
        }
        Legacy::Filesystem { .. } | Legacy::Io(_) => Error::ServerError {
            code: err.variant_str().into_owned(),
            description,
        },
        Legacy::Diag { code, .. } => classify_diag(code, description),
    }
}

fn classify_diag(code: &'static str, description: String) -> Error {
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
        | "source-remote-unsupported" => bad_request(code, description),
        "spec-not-generated" | "adapter-component-missing" => not_found(code, description),
        "source-extract-failed" | "claim-extras-malformed" | "synthesis-model-failed" => {
            bad_gateway(code, description)
        }
        _ => server_error(code, description),
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
