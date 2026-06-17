//! Closed failure mode for the hint interpreter.

use std::path::PathBuf;

use thiserror::Error;

use crate::rules::HintKind;

/// Closed failure mode for the hint interpreter.
///
/// Variants map to the lint exit mapping exit-code table at the handler boundary —
/// `Unsupported`, `SchemaCompile`, `SchemaResolve`, `RegexCompile`,
/// `Filesystem`, and `ToolInvocation` are infrastructure failures
/// the caller maps to `Error::Validation` (exit 2) or
/// `Error::Filesystem` (exit 1) per lint exit mapping. Recoverable per-finding
/// states (`tool.invocation-failed`, `tool.undeclared`, the reserved-hint diagnostics
/// summary) flow back as [`specify_diagnostics::Diagnostic`] entries on the Ok path.
#[derive(Debug, Error)]
pub enum HintError {
    /// Hint shape outside the v1 contract (reserved kinds called
    /// directly, `http(s)://` schema refs, glob negation, …).
    #[error("rule {rule_id}: hint kind {kind:?} unsupported: {reason}")]
    Unsupported {
        /// Originating rule id.
        rule_id: String,
        /// Hint kind that triggered the rejection.
        kind: HintKind,
        /// Static reason copied into operator-facing diagnostics.
        reason: &'static str,
    },
    /// JSON Schema referenced by a `kind: schema` hint failed to
    /// compile.
    #[error("rule {rule_id}: schema {schema_ref} failed to compile: {detail}")]
    SchemaCompile {
        /// Originating rule id.
        rule_id: String,
        /// Schema reference verbatim from the hint's `value`.
        schema_ref: String,
        /// Compiler error message.
        detail: String,
    },
    /// Schema reference could not be resolved (unknown registered id,
    /// missing project file, escapes `project_dir` via `..`,
    /// `http(s)://` ref).
    #[error("rule {rule_id}: schema {schema_ref} could not be resolved: {reason}")]
    SchemaResolve {
        /// Originating rule id.
        rule_id: String,
        /// Schema reference verbatim from the hint's `value`.
        schema_ref: String,
        /// Free-form resolution reason.
        reason: String,
    },
    /// Regex pattern carried by a `kind: regex` hint did not compile.
    #[error("rule {rule_id}: regex {pattern} failed to compile: {source}")]
    RegexCompile {
        /// Originating rule id.
        rule_id: String,
        /// Pattern verbatim from the hint's `value`.
        pattern: String,
        /// Underlying `regex` crate error.
        #[source]
        source: ::regex::Error,
    },
    /// Extension invocation failed at the runtime boundary (the WASI host
    /// could not invoke the declared tool). Recoverable
    /// non-zero-exit outcomes flow as `tool.invocation-failed`
    /// findings on the Ok path per `kind: tool` evaluator contract.
    #[error("rule {rule_id}: tool {tool} invocation failed: {detail}")]
    ToolInvocation {
        /// Originating rule id.
        rule_id: String,
        /// Extension name from the hint's `value`.
        tool: String,
        /// Free-form invocation failure detail.
        detail: String,
    },
    /// Reserved variant — `tool.undeclared` is emitted as a finding
    /// on the Ok path per `kind: tool` evaluator contract. The variant is preserved on the
    /// closed enum so callers exhaustively match every `kind: tool` evaluator contract-mandated
    /// surface.
    #[error("rule {rule_id}: tool {tool} not declared by the project")]
    ToolUndeclared {
        /// Originating rule id.
        rule_id: String,
        /// Extension name from the hint's `value`.
        tool: String,
    },
    /// Filesystem I/O against a candidate file failed during
    /// evaluation (the indexer normally skips unreadable files but
    /// races between scan and eval can still surface here).
    #[error("filesystem {op} on {path}: {source}", path = path.display())]
    Filesystem {
        /// Operation name (`read`, `parse`, …).
        op: &'static str,
        /// Path the operation targeted.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}
