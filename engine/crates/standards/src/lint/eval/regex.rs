//! `kind: regex` evaluator per the executable hint-kind contract.

mod config;
pub mod logical_lines;

use std::path::{Path, PathBuf};

use ::regex::Regex;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use self::config::RegexHintConfig;
use super::{HintError, make_finding};
use crate::lint::{FileKind, WorkspaceModel};
use crate::rules::{ResolvedRule, RuleHint};

const SNIPPET_MAX_CHARS: usize = 240;

#[expect(
    clippy::too_many_lines,
    reason = "regex eval handles normal, capture, and slash-skill modes in one pass"
)]
pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], project_dir: &Path,
    model: &WorkspaceModel, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let cfg = RegexHintConfig::parse(rule, hint)?;
    let pattern = if cfg.slash_skill_positional {
        None
    } else {
        Some(Regex::new(&hint.value).map_err(|err| HintError::RegexCompile {
            rule_id: rule.rule_id.clone(),
            pattern: hint.value.clone(),
            source: err,
        })?)
    };

    let mut out: Vec<Diagnostic> = Vec::new();
    for candidate in candidates {
        let candidate_str = candidate.to_string_lossy();
        if !is_text_file(model, &candidate_str) {
            continue;
        }
        let absolute = project_dir.join(candidate);
        let bytes = match std::fs::read(&absolute) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(HintError::Filesystem {
                    op: "read",
                    path: absolute,
                    source: err,
                });
            }
        };
        let text = String::from_utf8_lossy(&bytes);
        if cfg.slash_skill_positional {
            let line_iter: Vec<(usize, String)> = if cfg.join_backslash_continuations {
                logical_lines::logical_lines_with_starts(&text)
            } else {
                text.lines().enumerate().map(|(idx, line)| (idx + 1, line.to_string())).collect()
            };
            for (start_line, logical) in line_iter {
                if !logical_lines::violates_slash_skill_positional(&logical) {
                    continue;
                }
                let end_line = if cfg.join_backslash_continuations && logical.contains('\n') {
                    start_line + logical.lines().count().saturating_sub(1)
                } else {
                    start_line
                };
                let line_suffix =
                    if end_line > start_line { format!("-{end_line}") } else { String::new() };
                push_finding(
                    rule,
                    hint,
                    &mut out,
                    next_id,
                    &candidate_str,
                    start_line,
                    1,
                    &format!(
                        "Slash skill invocation uses flag-style arguments at line {start_line}{line_suffix}"
                    ),
                );
            }
            continue;
        }

        let pattern = pattern.as_ref().expect("slash-skill branch continues above");
        for (line_idx, line) in text.lines().enumerate() {
            if cfg.negative_match {
                if !pattern.is_match(line) {
                    push_finding(
                        rule,
                        hint,
                        &mut out,
                        next_id,
                        &candidate_str,
                        line_idx + 1,
                        1,
                        line,
                    );
                }
                continue;
            }

            if cfg.capture_group.is_some() {
                for capture in pattern.captures_iter(line) {
                    let m = capture.get(0).expect("captures_iter yields match 0");
                    if !match_passes(&cfg, line, m, &capture) {
                        continue;
                    }
                    push_finding(
                        rule,
                        hint,
                        &mut out,
                        next_id,
                        &candidate_str,
                        line_idx + 1,
                        m.start() + 1,
                        line,
                    );
                }
            } else {
                for m in pattern.find_iter(line) {
                    if !match_passes_find(&cfg, line, m) {
                        continue;
                    }
                    push_finding(
                        rule,
                        hint,
                        &mut out,
                        next_id,
                        &candidate_str,
                        line_idx + 1,
                        m.start() + 1,
                        line,
                    );
                }
            }
        }
    }
    Ok(out)
}

fn match_passes_find(cfg: &RegexHintConfig, line: &str, m: ::regex::Match<'_>) -> bool {
    if let Some(prefix) = cfg.suffix_must_not_start_with.as_deref()
        && line[m.end()..].starts_with(prefix)
    {
        return false;
    }
    true
}

fn match_passes(
    cfg: &RegexHintConfig, line: &str, m: ::regex::Match<'_>, capture: &::regex::Captures<'_>,
) -> bool {
    if !match_passes_find(cfg, line, m) {
        return false;
    }
    let Some(group) = cfg.capture_group else {
        return true;
    };
    let Some(cap) = capture.get(group as usize) else {
        return false;
    };
    let digits: i64 = cap
        .as_str()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(i64::MAX);
    cfg.capture_passes(digits)
}

#[expect(clippy::too_many_arguments, reason = "finding builder mirrors eval call sites")]
fn push_finding(
    rule: &ResolvedRule, hint: &RuleHint, out: &mut Vec<Diagnostic>, next_id: &mut u64, path: &str,
    line: usize, column: usize, line_text: &str,
) {
    let line_no = u32::try_from(line).unwrap_or(u32::MAX);
    let column_no = u32::try_from(column).unwrap_or(u32::MAX);
    let location = FindingLocation {
        path: path.to_string(),
        line: Some(line_no),
        column: Some(column_no),
        end_line: None,
        end_column: None,
    };
    let evidence = FindingEvidence::Snippet {
        value: clip_snippet(line_text),
    };
    let title =
        format!("{}: matched `{}`", rule.title, hint.description.as_deref().unwrap_or(&hint.value));
    let finding = make_finding(rule, *next_id, title, Some(location), evidence);
    *next_id += 1;
    out.push(finding);
}

fn is_text_file(model: &WorkspaceModel, candidate: &str) -> bool {
    model
        .files
        .iter()
        .find(|f| f.path == candidate)
        .is_none_or(|f| matches!(f.kind, FileKind::Text))
}

fn clip_snippet(line: &str) -> String {
    if line.chars().count() <= SNIPPET_MAX_CHARS {
        return line.to_owned();
    }
    let mut out = String::new();
    for ch in line.chars().take(SNIPPET_MAX_CHARS) {
        out.push(ch);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::evaluate;
    use crate::lint::{File, FileKind, WorkspaceModel};
    use crate::rules::{HintKind, Origin, PathRoot, ResolvedRule, RuleHint};

    fn rule() -> ResolvedRule {
        ResolvedRule {
            rule_id: "CORE-050".to_string(),
            title: "fixture".to_string(),
            severity: specify_diagnostics::Severity::Important,
            trigger: "t".to_string(),
            lint_mode: None,
            applicability: None,
            rule_hints: None,
            references: None,
            origin: Origin::Core,
            path_root: PathRoot::RulesRoot,
            path: "adapters/shared/rules/core/CORE-050.md".to_string(),
            body: String::new(),
            deprecated: None,
        }
    }

    #[test]
    fn suffix_guard_skips_validate_suffix() {
        let hint = RuleHint {
            kind: HintKind::Regex,
            value: r"\bspecify-contract\b".to_string(),
            description: None,
            config: Some(serde_json::json!({ "suffix-must-not-start-with": "-validate" })),
        };
        let mut model = WorkspaceModel::default();
        model.files.push(File {
            path: "plugins/spec/skills/x/SKILL.md".into(),
            kind: FileKind::Text,
            language: None,
            sha256: None,
        });
        let dir = tempfile::tempdir().expect("tempdir");
        let skill = dir.path().join("plugins/spec/skills/x/SKILL.md");
        std::fs::create_dir_all(skill.parent().expect("parent")).expect("mkdir");
        std::fs::write(&skill, "Run specify-contract-validate here\nAlso specify-contract alone\n")
            .expect("write");

        let rel = PathBuf::from("plugins/spec/skills/x/SKILL.md");
        let mut next_id = 0_u64;
        let findings =
            evaluate(&rule(), &hint, &[rel], dir.path(), &model, &mut next_id).expect("eval");
        let lines: Vec<u32> = findings.iter().filter_map(|f| f.location.as_ref()?.line).collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], 2);
    }

    #[test]
    fn binary_candidates_skipped() {
        let hint = RuleHint {
            kind: HintKind::Regex,
            value: r"https?://".to_string(),
            description: None,
            config: None,
        };
        let mut model = WorkspaceModel::default();
        model.files.push(File {
            path: "app.rs".into(),
            kind: FileKind::Text,
            language: None,
            sha256: None,
        });
        model.files.push(File {
            path: "blob.bin".into(),
            kind: FileKind::Binary,
            language: None,
            sha256: None,
        });
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("app.rs"), "fn url() { \"https://api.example.com\" }\n")
            .expect("write rs");
        std::fs::write(dir.path().join("blob.bin"), b"\x00\x00https://hidden\n")
            .expect("write bin");

        let cands = vec![PathBuf::from("app.rs"), PathBuf::from("blob.bin")];
        let mut next_id = 1_u64;
        let findings =
            evaluate(&rule(), &hint, &cands, dir.path(), &model, &mut next_id).expect("eval");
        assert_eq!(findings.len(), 1, "binary file must be skipped");
        let location = findings[0].location.as_ref().expect("location");
        assert_eq!(location.path, "app.rs");
        assert_eq!(location.line, Some(1));
        assert!(location.column.expect("column") >= 1);
        match &findings[0].evidence {
            specify_diagnostics::FindingEvidence::Snippet { value } => {
                assert!(value.contains("https://api.example.com"), "{value}");
            }
            other => panic!("expected snippet evidence, got {other:?}"),
        }
    }

    #[test]
    fn compile_failure_is_hard_error() {
        let hint = RuleHint {
            kind: HintKind::Regex,
            value: "(unclosed".to_string(),
            description: None,
            config: None,
        };
        let model = WorkspaceModel::default();
        let dir = tempfile::tempdir().expect("tempdir");
        let mut next_id = 1_u64;
        let err = evaluate(&rule(), &hint, &[], dir.path(), &model, &mut next_id)
            .expect_err("regex must fail to compile");
        assert!(format!("{err}").contains("regex"), "{err}");
    }
}
