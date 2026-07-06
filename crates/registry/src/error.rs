//! Extension-specific errors and conversions into the shared Specify error surface.

#![allow(
    clippy::needless_pass_by_value,
    reason = "Diag-helper constructors take owned errors so callers can `.map_err(|err| ExtensionError::cache_io(..., err))` without an extra `&` at every site."
)]

use std::path::PathBuf;

/// Errors produced by declared-tool manifest loading, validation, and cache
/// helpers.
///
/// Variants follow the Diag-first policy from `DECISIONS.md` §"Diag-first
/// error policy": a typed variant exists only when (a) a test or skill
/// destructures the payload, (b) the variant routes to a non-default
/// `Exit` slot via `From<ExtensionError> for Error`, or (c) three or more call
/// sites share the exact shape. Everything else lands on [`Self::Diag`]
/// with a kebab-case `code` carried at the constructor site (see the
/// `sidecar_*` / `network_*` helpers below).
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    /// Catch-all diagnostic. The `code` becomes the `error` field of the
    /// JSON envelope after [`From<ExtensionError> for specify_error::Error`];
    /// `detail` is the human-readable message.
    #[error("{detail}")]
    Diag {
        /// Kebab-case diagnostic code carried at the constructor site.
        code: &'static str,
        /// Human-readable message rendered into the envelope.
        detail: String,
    },
    /// The requested tool name was not present in merged declarations.
    #[error("tool not declared: {name}")]
    ToolNotDeclared {
        /// Extension name absent from the merged declaration set.
        name: String,
    },
    /// A tool permission template or expanded permission path is invalid.
    #[error("invalid tool permission `{template}`: {reason}")]
    InvalidPermission {
        /// Offending permission template or expanded path.
        template: String,
        /// Why the template or path is rejected.
        reason: String,
    },
    /// A requested preopen or filesystem authority is denied.
    #[error("tool permission denied for {}: {reason}", path.display())]
    PermissionDenied {
        /// Path whose access was denied.
        path: PathBuf,
        /// Why access to the path is denied.
        reason: String,
    },
}

impl ExtensionError {
    /// Build a cache or local-source I/O error. The single named helper
    /// keeps call sites readable across cache writes, resolver staging,
    /// and local-source reads.
    pub(crate) fn cache_io(
        action: &'static str, path: impl Into<PathBuf>, source: std::io::Error,
    ) -> Self {
        let path = path.into();
        Self::Diag {
            code: "tool-io",
            detail: format!("tool I/O failed while {action} {}: {source}", path.display()),
        }
    }

    /// Build a cache-root error. Surfaced when an environment-derived
    /// cache root is empty/relative or when a write target has no parent.
    pub(crate) fn cache_root(detail: impl Into<String>) -> Self {
        Self::Diag {
            code: "tool-cache-root",
            detail: format!("tool cache root error: {}", detail.into()),
        }
    }

    /// Build an atomic-cache-move error.
    pub(crate) fn atomic_move_failed(
        from: impl Into<PathBuf>, to: impl Into<PathBuf>, source: std::io::Error,
    ) -> Self {
        let from = from.into();
        let to = to.into();
        Self::Diag {
            code: "tool-atomic-move-failed",
            detail: format!(
                "atomic cache move failed from {} to {}: {source}",
                from.display(),
                to.display()
            ),
        }
    }

    /// Build a permission-shape error.
    #[must_use]
    pub fn invalid_permission(template: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidPermission {
            template: template.into(),
            reason: reason.into(),
        }
    }

    /// Build a permission-denied error.
    #[must_use]
    pub fn permission_denied(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::PermissionDenied {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Build a `tool-package` diagnostic for wasm-pkg fetch failures.
    pub(crate) fn package(
        request: &crate::manifest::PackageRequest, reason: impl Into<String>,
    ) -> Self {
        Self::Diag {
            code: "tool-package",
            detail: format!(
                "tool package `{}` failed: {}",
                request.to_wire_string(),
                reason.into()
            ),
        }
    }

    /// Build a `tool-package` diagnostic from a free-form source label.
    /// Used by the wasm-pkg config loader where the originating
    /// `PackageRequest` is no longer in scope.
    pub(crate) fn package_label(label: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Diag {
            code: "tool-package",
            detail: format!("tool package `{}` failed: {}", label.into(), reason.into()),
        }
    }

    /// Build a `tool-sidecar-parse` diagnostic.
    pub(crate) fn sidecar_parse(path: impl Into<PathBuf>, source: impl std::fmt::Display) -> Self {
        let path = path.into();
        Self::Diag {
            code: "tool-sidecar-parse",
            detail: format!("tool sidecar at {}: invalid YAML: {source}", path.display()),
        }
    }

    /// Build a `tool-sidecar-schema` diagnostic.
    pub(crate) fn sidecar_schema(path: impl Into<PathBuf>, detail: impl Into<String>) -> Self {
        let path = path.into();
        Self::Diag {
            code: "tool-sidecar-schema",
            detail: format!(
                "tool sidecar at {}: invalid schema: {}",
                path.display(),
                detail.into()
            ),
        }
    }

    /// Build a `tool-network-status` diagnostic.
    pub(crate) fn network_status(url: impl Into<String>, status: u16) -> Self {
        Self::Diag {
            code: "tool-network-status",
            detail: format!("`{}` returned HTTP status {status}; expected 200", url.into()),
        }
    }

    /// Build a `tool-network-timeout` diagnostic.
    pub(crate) fn network_timeout(url: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Diag {
            code: "tool-network-timeout",
            detail: format!("`{}` timed out: {}", url.into(), detail.into()),
        }
    }

    /// Build a `tool-network-malformed` diagnostic.
    pub(crate) fn network_malformed(url: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Diag {
            code: "tool-network-malformed",
            detail: format!("`{}` is malformed: {}", url.into(), detail.into()),
        }
    }

    /// Build a `tool-network-too-large` diagnostic.
    pub(crate) fn network_too_large(
        url: impl Into<String>, limit: u64, actual: Option<u64>,
    ) -> Self {
        let observed = actual.map_or_else(String::new, |size| format!(" (observed {size} bytes)"));
        Self::Diag {
            code: "tool-network-too-large",
            detail: format!("`{}` exceeded {limit} bytes{observed}", url.into()),
        }
    }

    /// Build a `tool-network-other` diagnostic.
    pub(crate) fn network_other(url: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Diag {
            code: "tool-network-other",
            detail: format!("`{}` network error: {}", url.into(), detail.into()),
        }
    }

    /// Build an `adapter-pack-failed` diagnostic for the byte-deterministic
    /// pack stage (RFC-48 D1).
    pub(crate) fn pack(detail: impl Into<String>) -> Self {
        Self::Diag {
            code: "adapter-pack-failed",
            detail: format!("adapter pack failed: {}", detail.into()),
        }
    }

    /// Build an `adapter-transport-failed` diagnostic for OCI
    /// publish/pull failures (RFC-48 D6).
    pub(crate) fn transport(reference: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Diag {
            code: "adapter-transport-failed",
            detail: format!(
                "adapter registry transport for `{}` failed: {}",
                reference.into(),
                detail.into()
            ),
        }
    }

    /// Build an `adapter-digest-mismatch` diagnostic — the verify-on-read
    /// gate that refuses an artifact whose content digest differs from
    /// the recorded immutable identity (RFC-48 D4).
    pub(crate) fn digest_mismatch(
        reference: impl Into<String>, expected: impl Into<String>, actual: impl Into<String>,
    ) -> Self {
        Self::Diag {
            code: "adapter-digest-mismatch",
            detail: format!(
                "adapter `{}` content digest {} does not match the recorded immutable digest {}",
                reference.into(),
                actual.into(),
                expected.into(),
            ),
        }
    }
}

impl From<ExtensionError> for specify_error::Error {
    fn from(value: ExtensionError) -> Self {
        match value {
            ExtensionError::Diag { code, detail } => Self::Diag { code, detail },
            err @ ExtensionError::ToolNotDeclared { .. } => Self::validation_failed(
                "tool-not-declared",
                "tool must be declared in project.yaml or by a bound adapter",
                err.to_string(),
            ),
            err @ (ExtensionError::InvalidPermission { .. }
            | ExtensionError::PermissionDenied { .. }) => Self::validation_failed(
                "tool-permission-denied",
                "tool must request permitted resources",
                err.to_string(),
            ),
        }
    }
}
