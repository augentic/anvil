//! Shared finding and arg plumbing for the in-process framework
//! checkers.
//!
//! Each checker returns [`ToolFinding`] rows; [`to_diagnostics`] maps
//! them to typed [`Diagnostic`] values the `kind: tool` evaluator folds
//! directly. The host restamps `id` and `fingerprint` after folding, so
//! both are placeholders here.

use serde_json::Value as JsonValue;
use specify_diagnostics::{
    Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, FindingEvidence, FindingLocation,
    Severity,
};

/// Placeholder fingerprint in the `sha256:<64 hex>` wire shape; the
/// evaluator recomputes it on fold.
const PLACEHOLDER_FINGERPRINT: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// One checker finding plus its operator-facing guidance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolFinding {
    /// Codex `CORE-NNN` id the finding belongs to.
    pub rule_id: &'static str,
    /// Project-relative, forward-slash path of the offending file, or
    /// `None` for whole-tree findings.
    pub path: Option<String>,
    /// Operator-facing message describing the violation.
    pub message: String,
    /// Operator-facing impact prose.
    pub impact: &'static str,
    /// Operator-facing remediation prose.
    pub remediation: &'static str,
}

/// Map checker findings to typed [`Diagnostic`] values — the direct
/// in-process path, with no JSON round-trip. The evaluator restamps
/// `id` and `fingerprint` on fold.
pub fn to_diagnostics(findings: &[ToolFinding]) -> Vec<Diagnostic> {
    findings.iter().enumerate().map(|(index, row)| diagnostic(index, row)).collect()
}

fn diagnostic(index: usize, row: &ToolFinding) -> Diagnostic {
    Diagnostic {
        id: format!("FIND-{:04}", index + 1),
        rule_id: Some(row.rule_id.to_string()),
        related_rule_ids: None,
        title: row.message.clone(),
        severity: Severity::Important,
        source: DiagnosticSource::Tool,
        kind: DiagnosticKind::Violation,
        target_adapter: None,
        source_adapter: None,
        slice: None,
        change: None,
        artifact: Artifact::Unknown,
        location: row.path.as_ref().map(|path| FindingLocation {
            path: path.clone(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        }),
        evidence: FindingEvidence::Snippet {
            value: row.message.clone(),
        },
        impact: row.impact.to_string(),
        remediation: row.remediation.to_string(),
        confidence: None,
        fingerprint: PLACEHOLDER_FINGERPRINT.to_string(),
        status: None,
        disposition: None,
    }
}

/// The single rule id from `rules` named by the invocation's candidate
/// path (the rule's own sentinel file), or `None` when the candidate
/// does not name a recognised rule.
///
/// Deliberately stricter than the retired WASI tools'
/// any-arg-substring scan: only the first positional arg (the candidate
/// path the evaluator always passes first) is consulted, and only its
/// file name — a forwarded `config:` JSON that happens to mention
/// another `CORE-NNN` can no longer mis-scope the invocation.
pub fn requested_rule(args: &[String], rules: &[&'static str]) -> Option<&'static str> {
    let candidate = args.first()?;
    let file_name = candidate.rsplit(['/', '\\']).next().unwrap_or(candidate);
    rules.iter().copied().find(|rule| file_name.contains(rule))
}

/// The first positional arg that parses as a JSON object — the rule's
/// `config:` forwarded by the `kind: tool` evaluator.
pub fn parsed_config(args: &[String]) -> Option<JsonValue> {
    args.iter().find_map(|arg| match serde_json::from_str::<JsonValue>(arg) {
        Ok(value) if value.is_object() => Some(value),
        _ => None,
    })
}

/// Numeric field accessor over the forwarded config; `0` when absent.
pub fn usize_field(config: Option<&JsonValue>, key: &str) -> usize {
    config
        .and_then(|value| value.get(key))
        .and_then(JsonValue::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0)
}

/// String-array field accessor over the forwarded config; empty when
/// absent.
pub fn string_array_field(config: Option<&JsonValue>, key: &str) -> Vec<String> {
    config
        .and_then(|value| value.get(key))
        .and_then(JsonValue::as_array)
        .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Display `path` relative to `root` with forward slashes.
pub fn relative_display(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// Recursive file collector that never follows or records symlinks,
/// matching the retired tools' `follow_links(false)` + symlink-skip
/// discovery posture.
pub fn walk_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk_files(&path, out);
        } else if file_type.is_file() {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_rule_reads_candidate_only() {
        let args = vec![
            "adapters/shared/rules/core/CORE-026-duplicate-rule-id.md".to_string(),
            r#"{"note": "mentions CORE-009 in config"}"#.to_string(),
        ];
        assert_eq!(requested_rule(&args, &["CORE-009", "CORE-026"]), Some("CORE-026"));
    }

    #[test]
    fn to_diagnostics_maps_rule_id_and_severity() {
        let findings = vec![ToolFinding {
            rule_id: "CORE-026",
            path: Some("adapters/x.md".to_string()),
            message: "duplicate id".to_string(),
            impact: "impact",
            remediation: "fix it",
        }];
        let diagnostics = to_diagnostics(&findings);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id.as_deref(), Some("CORE-026"));
        assert_eq!(diagnostics[0].severity, Severity::Important);
    }
}
