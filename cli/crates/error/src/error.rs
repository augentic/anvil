//! `Error` enum and saphyr-error conversions.
//!
//! The `YamlDe` / `YamlSer` variants flatten `serde_saphyr`'s two error
//! types directly into the crate's error surface; callers that don't
//! care which API tripped can continue to `?`-propagate.

use std::borrow::Cow;

/// Structured error type for all `specify-*` crates.
///
/// Variants carry enough context for the CLI to assign exit codes and
/// choose an output format without string-parsing.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The `.specify/project.yaml` file is missing.
    #[error("not initialized: .specify/project.yaml not found")]
    NotInitialized,

    /// Structured catch-all for diagnostics that don't have a dedicated
    /// variant. The `code` is a stable kebab-case discriminant surfaced
    /// in JSON envelopes; `detail` is the human-readable message.
    /// Promote a recurring `Diag` site to its own variant once the call
    /// shape stabilises.
    #[error("{code}: {detail}")]
    Diag {
        /// Stable kebab-case discriminant surfaced as the JSON `error` field.
        code: &'static str,
        /// Human-readable message.
        detail: String,
    },

    /// A user-supplied CLI argument is invalid for reasons clap cannot
    /// catch (kebab-case names, mutually exclusive flag combinations,
    /// unknown enum keys, etc.). Carries the offending flag/value plus a
    /// human-readable detail. Prefer this over [`Error::Diag`] for
    /// argument-shape validation so the CLI can map it onto the
    /// argument-error exit code.
    #[error("invalid argument {flag}: {detail}")]
    Argument {
        /// Argument name (e.g. `--adapter`, `<name>`, `phase`).
        flag: &'static str,
        /// Human-readable explanation, suitable for display alongside `--help`.
        detail: String,
    },

    /// A workflow-gating validation surface failed. Payload-free: the
    /// rendered findings (a `DiagnosticReport`) are emitted to stdout by
    /// the handler; this variant only carries the stable kebab `code`
    /// (the JSON `error` discriminant) and a human-readable `detail`,
    /// and routes to exit code 2 (`Exit::ValidationFailed`). Construct
    /// via [`Self::validation_failed`].
    #[error("{code}: {detail}")]
    Validation {
        /// Stable kebab-case discriminant surfaced as the JSON `error`
        /// field. `Cow` so the common literal-code path borrows a
        /// `&'static str` (no per-construction or per-render allocation)
        /// while dynamic codes can still own a `String`.
        code: Cow<'static, str>,
        /// Human-readable message.
        detail: String,
    },

    /// The installed CLI version is older than the project floor.
    #[error("specify version {found} is older than the project floor {required}; upgrade the CLI")]
    CliTooOld {
        /// Minimum version the project requires.
        required: String,
        /// Version currently installed.
        found: String,
    },

    /// The installed CLI version is older than an adapter's declared
    /// host-CLI compatibility floor (RFC-47 D3, the `specify` manifest
    /// key). Routes to exit 3 (`Exit::VersionTooOld`) like
    /// [`Self::CliTooOld`] but carries the distinct `adapter-cli-too-old`
    /// discriminant so the operator sees which adapter outran the binary.
    #[error(
        "specify version {found} is older than the floor {required} required by adapter {adapter}; upgrade the CLI"
    )]
    AdapterCliTooOld {
        /// Adapter identity (and manifest path) that declared the floor.
        adapter: String,
        /// Minimum version the adapter requires.
        required: String,
        /// Version currently installed.
        found: String,
    },

    /// A required artifact was not found at the expected path.
    #[error("{kind} not found at {}", path.display())]
    ArtifactNotFound {
        /// Kind of artifact (e.g. `metadata.yaml`).
        kind: &'static str,
        /// Path where the artifact was expected.
        path: std::path::PathBuf,
    },

    /// A filesystem operation failed. The `op` field is a stable
    /// kebab-case suffix that, prefixed with `filesystem-`, becomes the
    /// JSON envelope's `error` discriminant (e.g. `filesystem-readdir`).
    /// Canonical call sites: the slice-merge engine
    /// (`specify_workflow::merge::slice::{read, write}`), where every
    /// recursive directory walk and file copy needs a stable, testable
    /// discriminant for operator follow-up.
    #[error("filesystem-{op}: {} ({source})", path.display())]
    Filesystem {
        /// Operation kind; the JSON discriminant is `filesystem-<op>`.
        op: &'static str,
        /// Path the operation acted on (or attempted to).
        path: std::path::PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// `specify workspace prepare` refused to land a branch
    /// because `specify_registry::branch::prepare` returned a
    /// diagnostic. The renderer surfaces the diagnostic key + paths
    /// alongside the human-readable detail.
    #[error("branch-preparation-failed: project `{project}`: {detail} ({key})")]
    BranchPrepareFailed {
        /// Project (registry slot) name.
        project: String,
        /// Stable diagnostic key from `specify_registry::branch`.
        key: String,
        /// Human-readable diagnostic message.
        detail: String,
        /// Repository-relative paths the diagnostic points at (may be
        /// empty when the diagnostic is whole-clone scoped).
        paths: Vec<String>,
    },

    /// An I/O error propagated from the standard library.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A YAML deserialization error (e.g. `serde_saphyr::from_str`).
    /// Library crates rely on `?`-propagation; the variant docstring is
    /// the canonical "you don't have to care which `serde_saphyr` API
    /// tripped" — match on either YAML variant when that distinction is
    /// irrelevant.
    #[error(transparent)]
    YamlDe(#[from] serde_saphyr::Error),

    /// A YAML serialization error (e.g. `serde_saphyr::to_string`).
    #[error(transparent)]
    YamlSer(#[from] serde_saphyr::ser::Error),
}

impl Error {
    /// Long-form recovery hint for tightened diagnostics. Returns
    /// `None` when the variant has no actionable follow-up beyond the
    /// `#[error("…")]` body.
    ///
    /// The renderer in `src/runtime/output.rs` calls this to surface guidance
    /// alongside the kebab discriminant on a TTY, while keeping the
    /// machine-readable JSON envelope compact. New hints land here
    /// (typed-arm for typed variants; `Self::Diag { code, .. }` arm for
    /// `Diag`-routed sites), not in the renderer.
    #[must_use]
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::Diag { code, .. } => match *code {
                "plan-has-outstanding-work" => Some(
                    "complete or drop the listed entries, or rerun with --force to archive anyway.",
                ),
                "init-requires-adapter-or-workspace" => Some(
                    "`specify init <adapter>` for a regular project, or `specify init --workspace` for a workspace.\nsee: docs/init.md",
                ),
                "context-existing-unfenced-agents-md" => {
                    Some("rerun with --force to rewrite AGENTS.md.")
                }
                "context-fenced-content-modified" => Some(
                    "reconcile the edits or rerun with --force to replace the generated block.",
                ),
                _ => None,
            },
            _ => None,
        }
    }

    /// Kebab-case identifier used in structured CLI error payloads.
    ///
    /// Most arms borrow a `&'static str` literal at zero cost;
    /// [`Self::Filesystem`] is the lone owned arm, composing
    /// `filesystem-<op>`.
    #[must_use]
    pub fn variant_str(&self) -> Cow<'static, str> {
        match self {
            Self::NotInitialized => Cow::Borrowed("not-initialized"),
            Self::Diag { code, .. } => Cow::Borrowed(*code),
            Self::Argument { .. } => Cow::Borrowed("argument"),
            Self::Validation { code, .. } => code.clone(),
            Self::CliTooOld { .. } => Cow::Borrowed("specify-version-too-old"),
            Self::AdapterCliTooOld { .. } => Cow::Borrowed("adapter-cli-too-old"),
            Self::ArtifactNotFound { .. } => Cow::Borrowed("artifact-not-found"),
            Self::Filesystem { op, .. } => Cow::Owned(format!("filesystem-{op}")),
            Self::BranchPrepareFailed { .. } => Cow::Borrowed("branch-preparation-failed"),
            Self::Io(_) => Cow::Borrowed("io"),
            Self::YamlDe(_) | Self::YamlSer(_) => Cow::Borrowed("yaml"),
        }
    }

    /// Build a payload-free `Validation` failure that lands on
    /// `Exit::ValidationFailed` (exit 2).
    ///
    /// `code` is the stable kebab discriminant surfaced as the JSON
    /// `error` field (and by [`Self::variant_str`]); `rule` (the
    /// human-readable invariant) and `detail` (the specific
    /// explanation) are folded into the rendered message. This is the
    /// operational counterpart to the rich [`Diagnostic`] report a
    /// handler renders on stdout — use it when the failure is a single
    /// operational signal (malformed YAML, unknown slice name, …)
    /// rather than a set of findings.
    ///
    /// [`Diagnostic`]: https://docs.rs/specify-diagnostics
    #[must_use]
    pub fn validation_failed(
        code: impl Into<Cow<'static, str>>, rule: impl Into<String>, detail: impl Into<String>,
    ) -> Self {
        let rule = rule.into();
        let detail = detail.into();
        let detail = if rule.is_empty() { detail } else { format!("{rule}: {detail}") };
        Self::Validation {
            code: code.into(),
            detail,
        }
    }
}

#[cfg(test)]
mod tests {
    // Exit-code mapping for these variants is locked down by the binary's
    // `tests/cli/errors.rs`; these unit tests cover the type's own
    // surface — the kebab discriminant (`variant_str`), the `Display`
    // body, and `hint` — without duplicating the `Exit::from` table.
    use super::Error;

    #[test]
    fn diag_round_trip() {
        let err = Error::Diag {
            code: "kebab-prefix",
            detail: "specific detail".to_string(),
        };
        assert_eq!(err.variant_str(), "kebab-prefix");
        assert_eq!(err.to_string(), "kebab-prefix: specific detail");
    }

    #[test]
    fn cli_too_old_discriminant_display() {
        let err = Error::CliTooOld {
            required: "1.0.0".to_string(),
            found: "0.9.0".to_string(),
        };
        assert_eq!(err.variant_str(), "specify-version-too-old");
        let msg = err.to_string();
        assert!(msg.contains("0.9.0") && msg.contains("1.0.0"), "both versions in display: {msg}");
        assert!(err.hint().is_none(), "CliTooOld has no recovery hint");
    }

    #[test]
    fn adapter_cli_too_old_discriminant_display() {
        // RFC-47 D3: the adapter floor reuses the exit-3 family but carries
        // a distinct discriminant naming the adapter that outran the binary.
        let err = Error::AdapterCliTooOld {
            adapter: "omnia (adapter.yaml)".to_string(),
            required: "2.0.0".to_string(),
            found: "1.0.0".to_string(),
        };
        assert_eq!(err.variant_str(), "adapter-cli-too-old");
        let msg = err.to_string();
        assert!(
            msg.contains("1.0.0") && msg.contains("2.0.0") && msg.contains("omnia"),
            "versions and adapter in display: {msg}"
        );
        assert!(err.hint().is_none(), "AdapterCliTooOld has no recovery hint");
    }

    #[test]
    fn validation_static_code_and_display() {
        // The common path borrows a `&'static str` code, and
        // `validation_failed` folds `rule` + `detail` into one message.
        let err = Error::validation_failed("bad-thing", "rule", "detail");
        assert_eq!(err.variant_str(), "bad-thing");
        assert_eq!(err.to_string(), "bad-thing: rule: detail");
    }

    #[test]
    fn validation_empty_rule_omits_prefix() {
        // Edge: an empty `rule` must not leave a dangling `": "` prefix.
        let err = Error::validation_failed("code", "", "just detail");
        assert_eq!(err.to_string(), "code: just detail");
        assert_eq!(err.variant_str(), "code");
    }
}
