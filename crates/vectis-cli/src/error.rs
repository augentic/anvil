//! Unified error types for the vectis CLI.
//!
//! Every subcommand handler returns `Result<serde_json::Value, VectisError>`. On
//! success the handler emits its result JSON to stdout and exits 0. On failure
//! the dispatcher serializes [`VectisError::to_json`] to stdout and exits with
//! the variant's [`VectisError::exit_code`].

use serde::Serialize;
use std::io;
use thiserror::Error;

/// A single missing tool reported by the prerequisite checker.
///
/// Matches the shape documented in RFC-6 § Prerequisite Detection.
#[derive(Debug, Clone, Serialize)]
pub struct MissingTool {
    pub tool: String,
    pub assembly: String,
    pub check: String,
    pub install: String,
}

/// All terminal failure modes for the CLI.
///
/// Subcommand handlers convert their internal errors into one of these
/// variants. The dispatcher turns the variant into the RFC's structured JSON
/// error shape via [`VectisError::to_json`].
///
/// Every variant is actively constructed today: chunks 1/2 build
/// `MissingPrerequisites`, `Io`, and `InvalidProject`; chunk 4 constructs
/// `Internal` (from `versions::load_embedded` if the embedded defaults
/// are ever malformed); chunks 7/8/9 construct `Verify` from the
/// per-assembly build pipelines.
#[derive(Debug, Error)]
pub enum VectisError {
    #[error("missing prerequisites: {message}")]
    MissingPrerequisites {
        missing: Vec<MissingTool>,
        message: String,
    },

    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid project: {message}")]
    InvalidProject { message: String },

    #[error("verify failed: {message}")]
    Verify { message: String },

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl VectisError {
    /// Process exit code for this error.
    ///
    /// Missing prerequisites is `2` so callers can distinguish "your
    /// workstation is incomplete" from generic failure (`1`).
    pub fn exit_code(&self) -> i32 {
        match self {
            VectisError::MissingPrerequisites { .. } => 2,
            _ => 1,
        }
    }

    /// Render the error as the structured JSON shape defined in RFC-6.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            VectisError::MissingPrerequisites { missing, message } => serde_json::json!({
                "error": "missing_prerequisites",
                "missing": missing,
                "message": message,
            }),
            VectisError::Io(err) => serde_json::json!({
                "error": "io",
                "message": err.to_string(),
            }),
            VectisError::InvalidProject { message } => serde_json::json!({
                "error": "invalid_project",
                "message": message,
            }),
            VectisError::Verify { message } => serde_json::json!({
                "error": "verify",
                "message": message,
            }),
            VectisError::Internal { message } => serde_json::json!({
                "error": "internal",
                "message": message,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_prerequisites_json_shape() {
        let err = VectisError::MissingPrerequisites {
            missing: vec![MissingTool {
                tool: "xcodegen".into(),
                assembly: "ios".into(),
                check: "xcodegen --version".into(),
                install: "brew install xcodegen".into(),
            }],
            message: "Install the missing tools above and re-run the command.".into(),
        };
        let v = err.to_json();
        assert_eq!(v["error"], "missing_prerequisites");
        assert_eq!(v["missing"][0]["tool"], "xcodegen");
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn invalid_project_json_shape() {
        let err = VectisError::InvalidProject {
            message: "version file not found: /nonexistent.toml".into(),
        };
        let v = err.to_json();
        assert_eq!(v["error"], "invalid_project");
        assert_eq!(v["message"], "version file not found: /nonexistent.toml");
        assert_eq!(err.exit_code(), 1);
    }
}
