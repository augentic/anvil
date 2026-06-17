//! Lint-surface diagnostic glue.
//!
//! The neutral diagnostic substrate — [`specify_diagnostics::DiagnosticReport`]
//! envelope, [`specify_diagnostics::Format`], renderers, and
//! [`specify_diagnostics::RenderError`] — lives in the
//! [`specify_diagnostics`] leaf. Import those types directly from the
//! leaf; this module keeps lint-specific glue the leaf must not carry:
//! the `WorkspaceModel` dump path and the `IndexError` / `HintError` /
//! [`specify_diagnostics::RenderError`] → [`Error`] mappers that pin the
//! lint exit-code table.

use specify_diagnostics::RenderError;
use specify_error::{Error, Result};
use specify_schema::{WORKSPACE_MODEL_JSON_SCHEMA, validate_serialisable};

use crate::lint::WorkspaceModel;
use crate::lint::eval::HintError;
use crate::lint::index::IndexError;
use crate::rules::ResolvedRule;

/// Serialise the model, validate it against the v1 schema, and print
/// it to stdout. Validation failure is an internal bug — wrapped as
/// `Error::Diag` (exit 1) per lint exit mapping.
///
/// # Errors
///
/// - `Error::Validation` when the serialised model fails the
///   [`WORKSPACE_MODEL_JSON_SCHEMA`] v1 schema.
/// - `Error::Diag { review-dump-model-serialise }` when JSON
///   serialisation itself fails.
pub fn emit_dump_model(model: &WorkspaceModel) -> Result<()> {
    validate_serialisable(
        model,
        WORKSPACE_MODEL_JSON_SCHEMA,
        "review-dump-model-schema",
        "WorkspaceModel matches workspace-model.schema.json",
        "review-dump-model-serialise",
        "WorkspaceModel",
    )?;
    let rendered = serde_json::to_string_pretty(model).map_err(|err| Error::Diag {
        code: "review-dump-model-serialise",
        detail: format!("failed to serialise WorkspaceModel: {err}"),
    })?;
    println!("{rendered}");
    Ok(())
}

/// Map a `lint::index::IndexError` onto the lint exit mapping exit-code table.
///
/// | `IndexError`                | `Error` variant                            | Exit |
/// |-----------------------------|--------------------------------------------|------|
/// | `UnsupportedScanProfile`    | `Validation { review-unsupported-scan-profile }` | 2 |
/// | `ProjectDirMissing`         | `Validation { review-project-dir-missing }`      | 2 |
/// | `OverrideCompile`           | `Validation { review-index-override-compile }`   | 2 |
/// | `Filesystem`                | `Validation { review-index-filesystem }`         | 2 |
#[must_use]
pub fn map_index_error(err: IndexError) -> Error {
    match err {
        IndexError::UnsupportedScanProfile(profile) => Error::validation_failed(
            "review-unsupported-scan-profile",
            "scan profile is not supported",
            format!("requested scan profile: {profile:?}"),
        ),
        IndexError::ProjectDirMissing(path) => Error::validation_failed(
            "review-project-dir-missing",
            "project directory does not exist",
            path.display().to_string(),
        ),
        IndexError::Filesystem(detail) => Error::validation_failed(
            "review-index-filesystem",
            "filesystem error during indexer walk",
            detail,
        ),
        IndexError::OverrideCompile(detail) => Error::validation_failed(
            "review-index-override-compile",
            "always-ignore override pattern failed to compile",
            detail,
        ),
    }
}

/// Map a `lint::eval::HintError` onto the lint exit mapping exit-code table.
///
/// | `HintError`        | `Error` variant                                  | Exit |
/// |--------------------|--------------------------------------------------|------|
/// | `Unsupported`      | `Validation { review-unsupported-hint-kind }`    | 2    |
/// | `SchemaCompile`    | `Validation { review-schema-compile-failed }`    | 2    |
/// | `SchemaResolve`    | `Validation { review-schema-resolve-failed }`    | 2    |
/// | `RegexCompile`     | `Validation { review-regex-compile-failed }`     | 2    |
/// | `ToolInvocation`   | `Validation { review-tool-invocation-failed }`   | 2    |
/// | `ToolUndeclared`   | `Validation { review-tool-undeclared }`          | 2    |
/// | `Filesystem`       | `Filesystem { op: "review-eval" }`               | 1    |
#[must_use]
pub fn map_hint_error(rule: &ResolvedRule, err: HintError) -> Error {
    match err {
        HintError::Unsupported {
            rule_id,
            kind,
            reason,
        } => Error::validation_failed(
            "review-unsupported-hint-kind",
            format!("rule {rule_id}: hint kind {kind:?} is not supported in v1"),
            reason.to_string(),
        ),
        HintError::SchemaCompile {
            rule_id,
            schema_ref,
            detail,
        } => Error::validation_failed(
            "review-schema-compile-failed",
            format!("rule {rule_id}: schema {schema_ref} failed to compile"),
            detail,
        ),
        HintError::SchemaResolve {
            rule_id,
            schema_ref,
            reason,
        } => Error::validation_failed(
            "review-schema-resolve-failed",
            format!("rule {rule_id}: schema {schema_ref} could not be resolved"),
            reason,
        ),
        HintError::RegexCompile {
            rule_id,
            pattern,
            source,
        } => Error::validation_failed(
            "review-regex-compile-failed",
            format!("rule {rule_id}: regex {pattern} failed to compile"),
            source.to_string(),
        ),
        HintError::ToolInvocation {
            rule_id,
            tool,
            detail,
        } => Error::validation_failed(
            "review-tool-invocation-failed",
            format!("rule {rule_id}: tool {tool} invocation failed"),
            detail,
        ),
        HintError::ToolUndeclared { rule_id, tool } => Error::validation_failed(
            "review-tool-undeclared",
            format!("rule {rule_id}: tool {tool} not declared by the project"),
            format!("declare {tool} in project.yaml or remove the hint (rule path: {})", rule.path),
        ),
        HintError::Filesystem { path, source, .. } => Error::Filesystem {
            op: "review-eval",
            path,
            source,
        },
    }
}

/// Map a [`specify_diagnostics::RenderError`] onto the lint exit mapping exit-code table.
///
/// Both variants are internal bugs (the typed envelope cannot
/// legally fail v1 schema validation or JSON serialisation); the
/// mapping exists so the failure surface is uniform.
///
/// | `RenderError`              | `Error` variant                             | Exit |
/// |----------------------------|---------------------------------------------|------|
/// | `JsonSchemaValidation`     | `Diag { review-envelope-schema }`           | 1    |
/// | `JsonSerialise`            | `Diag { review-envelope-serialise }`        | 1    |
#[must_use]
pub fn map_render_error(err: RenderError) -> Error {
    match err {
        RenderError::JsonSchemaValidation { detail } => Error::Diag {
            code: "review-envelope-schema",
            detail,
        },
        RenderError::JsonSerialise(source) => Error::Diag {
            code: "review-envelope-serialise",
            detail: source.to_string(),
        },
        // `RenderError` is `#[non_exhaustive]` in the leaf; any future
        // variant maps to the same internal-bug envelope code.
        other => Error::Diag {
            code: "review-envelope-render",
            detail: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use specify_diagnostics::{RenderError, Severity};
    use specify_error::Error;

    use super::{map_hint_error, map_index_error, map_render_error};
    use crate::lint::ScanProfile;
    use crate::lint::eval::HintError;
    use crate::lint::index::IndexError;
    use crate::rules::{HintKind, Origin, PathRoot, ResolvedRule};

    const RULE_ID: &str = "UNI-001";

    fn fake_rule() -> ResolvedRule {
        ResolvedRule {
            rule_id: RULE_ID.into(),
            title: "t".into(),
            severity: Severity::Important,
            trigger: "trigger".into(),
            lint_mode: None,
            applicability: None,
            rule_hints: None,
            references: None,
            origin: Origin::Shared,
            path_root: PathRoot::RulesRoot,
            path: "shared/UNI-001.md".into(),
            body: String::new(),
            deprecated: None,
        }
    }

    struct ValidationCase<E> {
        err: fn() -> E,
        rule_id: &'static str,
    }

    struct DiagCase {
        err: fn() -> RenderError,
        code: &'static str,
    }

    fn hint_regex_compile() -> HintError {
        let pattern = "(".to_string();
        let source = ::regex::Regex::new(&pattern).expect_err("invalid regex");
        HintError::RegexCompile {
            rule_id: RULE_ID.into(),
            pattern,
            source,
        }
    }

    const INDEX_CASES: &[ValidationCase<IndexError>] = &[
        ValidationCase {
            err: || IndexError::UnsupportedScanProfile(ScanProfile::Framework),
            rule_id: "review-unsupported-scan-profile",
        },
        ValidationCase {
            err: || IndexError::ProjectDirMissing(PathBuf::from("/missing")),
            rule_id: "review-project-dir-missing",
        },
        ValidationCase {
            err: || IndexError::OverrideCompile("bad glob".into()),
            rule_id: "review-index-override-compile",
        },
        ValidationCase {
            err: || IndexError::Filesystem("symlink cycle at <link>".into()),
            rule_id: "review-index-filesystem",
        },
    ];

    const HINT_VALIDATION_CASES: &[ValidationCase<HintError>] = &[
        ValidationCase {
            err: || HintError::Unsupported {
                rule_id: RULE_ID.into(),
                kind: HintKind::SetCoverage,
                reason: "reserved",
            },
            rule_id: "review-unsupported-hint-kind",
        },
        ValidationCase {
            err: || HintError::SchemaCompile {
                rule_id: RULE_ID.into(),
                schema_ref: "rule".into(),
                detail: "compile failed".into(),
            },
            rule_id: "review-schema-compile-failed",
        },
        ValidationCase {
            err: || HintError::SchemaResolve {
                rule_id: RULE_ID.into(),
                schema_ref: "missing".into(),
                reason: "no such id".into(),
            },
            rule_id: "review-schema-resolve-failed",
        },
        ValidationCase {
            err: hint_regex_compile,
            rule_id: "review-regex-compile-failed",
        },
        ValidationCase {
            err: || HintError::ToolInvocation {
                rule_id: RULE_ID.into(),
                tool: "contract".into(),
                detail: "runtime".into(),
            },
            rule_id: "review-tool-invocation-failed",
        },
        ValidationCase {
            err: || HintError::ToolUndeclared {
                rule_id: RULE_ID.into(),
                tool: "contract".into(),
            },
            rule_id: "review-tool-undeclared",
        },
    ];

    const RENDER_CASES: &[DiagCase] = &[
        DiagCase {
            err: || RenderError::JsonSchemaValidation {
                detail: "schema mismatch".into(),
            },
            code: "review-envelope-schema",
        },
        DiagCase {
            err: || {
                RenderError::JsonSerialise(
                    serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
                )
            },
            code: "review-envelope-serialise",
        },
    ];

    fn assert_validation_rule_id(got: Error, rule_id: &str) {
        match got {
            Error::Validation { code, .. } => assert_eq!(code, rule_id),
            other => panic!("lint exit mapping: expected Validation({rule_id}), got {other:?}"),
        }
    }

    /// Every `IndexError` / `HintError` / `RenderError` variant maps
    /// onto its lint exit-mapping `Error` discriminant (the D8 table).
    #[test]
    fn error_mapping_matches_d8_table() {
        let rule = fake_rule();
        for case in INDEX_CASES {
            assert_validation_rule_id(map_index_error((case.err)()), case.rule_id);
        }
        for case in HINT_VALIDATION_CASES {
            assert_validation_rule_id(map_hint_error(&rule, (case.err)()), case.rule_id);
        }
        match map_hint_error(
            &rule,
            HintError::Filesystem {
                op: "read",
                path: PathBuf::from("/missing"),
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            },
        ) {
            Error::Filesystem { op, path, .. } => {
                assert_eq!(op, "review-eval");
                assert_eq!(path, PathBuf::from("/missing"));
            }
            other => panic!("lint exit mapping: expected Filesystem, got {other:?}"),
        }
        for case in RENDER_CASES {
            match map_render_error((case.err)()) {
                Error::Diag { code, .. } => assert_eq!(code, case.code),
                other => panic!("lint exit mapping: expected Diag({}), got {other:?}", case.code),
            }
        }
    }
}
