//! Structured errors and YAML conversions.

use std::borrow::Cow;

/// Workspace error type with structured CLI routing context.
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs, reason = "variant-level docs cover self-explanatory error fields")]
pub enum Error {
    /// Catch-all with a stable kebab-case code.
    ///
    /// Promote recurring, stable call shapes to dedicated variants.
    #[error("{code}: {detail}")]
    Diag { code: &'static str, detail: String },

    /// Argument validation that clap cannot express.
    ///
    /// Prefer this over [`Error::Diag`] for argument-error exit routing.
    #[error("invalid argument {flag}: {detail}")]
    Argument { flag: &'static str, detail: String },

    /// Payload-free validation failure routed to exit code 2.
    ///
    /// Findings are emitted separately; construct with [`Self::validation_failed`].
    #[error("{code}: {detail}")]
    Validation { code: Cow<'static, str>, detail: String },

    /// The CLI is older than an adapter's declared compatibility floor.
    ///
    /// Routes to exit 3 with the distinct `adapter-cli-too-old` code.
    #[error(
        "emery version {found} is older than the floor {required} required by adapter {adapter}; upgrade the CLI"
    )]
    AdapterCliTooOld { adapter: String, required: String, found: String },

    /// A required artifact was not found at the expected path.
    #[error("{kind} not found at {}", path.display())]
    ArtifactNotFound { kind: &'static str, path: std::path::PathBuf },

    /// Filesystem failure with a `filesystem-<op>` error code.
    #[error("filesystem-{op}: {} ({source})", path.display())]
    Filesystem {
        op: &'static str,
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An I/O error propagated from the standard library.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A YAML deserialization error.
    #[error(transparent)]
    YamlDe(#[from] serde_saphyr::Error),

    /// A YAML serialization error.
    #[error(transparent)]
    YamlSer(#[from] serde_saphyr::ser::Error),
}

impl Error {
    /// Return an actionable recovery hint, if one exists.
    ///
    /// Hints belong here, not in the renderer.
    #[must_use]
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::AdapterCliTooOld { .. } => Some(
                "update the installed binary through its install channel: `brew upgrade emery`, or `cargo install --git https://github.com/augentic/emery --locked`",
            ),
            Self::Validation { code, .. } if code.as_ref() == "specify-source-required" => Some(
                "`emery specify <adapter>...` generates the spec over the sources named on the invocation; there is no persisted binding list",
            ),
            Self::Diag {
                code: "spec-not-generated",
                ..
            } => Some("run `emery specify <adapter>...` to commit a generation, then re-run show"),
            _ => None,
        }
    }

    /// Kebab-case identifier for structured CLI errors.
    #[must_use]
    pub fn variant_str(&self) -> Cow<'static, str> {
        match self {
            Self::Diag { code, .. } => Cow::Borrowed(*code),
            Self::Argument { .. } => Cow::Borrowed("argument"),
            Self::Validation { code, .. } => code.clone(),
            Self::AdapterCliTooOld { .. } => Cow::Borrowed("adapter-cli-too-old"),
            Self::ArtifactNotFound { .. } => Cow::Borrowed("artifact-not-found"),
            Self::Filesystem { op, .. } => Cow::Owned(format!("filesystem-{op}")),
            Self::Io(_) => Cow::Borrowed("io"),
            Self::YamlDe(_) | Self::YamlSer(_) => Cow::Borrowed("yaml"),
        }
    }

    /// Build a payload-free validation failure for exit code 2.
    ///
    /// `code` is stable; `rule` and `detail` form the rendered message.
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
