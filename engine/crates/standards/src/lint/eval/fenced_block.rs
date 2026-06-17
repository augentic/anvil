//! `kind: fenced-block` evaluator.
//!
//! Consumes [`crate::lint::FencedBlock`] facts from the indexer and
//! applies closed `value` source discriminators:
//!
//! - `skill-envelope-json-in-body` (CORE-037) — flags JSON fences whose
//!   body looks like a CLI output envelope (heuristic, no config).
//! - `inline-json-too-long` (CORE-039) — flags fences whose info string
//!   is one of `config.langs` and whose body exceeds `config.max-lines`.
//!   Both the language allow-list and the line cap are **policy supplied
//!   by the rule file**, never a `const` in this arm (per the
//!   standards-layer policy-in-`specify` rule).
//! - `fenced-body-contains` (CORE-017) — flags fences whose info string
//!   is one of `config.langs` and whose body contains any of
//!   `config.substrings`. Both the language allow-list and the banned
//!   substring set are **policy supplied by the rule file**.

use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_SKILL_ENVELOPE_JSON_IN_BODY: &str = "skill-envelope-json-in-body";
const SOURCE_INLINE_JSON_TOO_LONG: &str = "inline-json-too-long";
const SOURCE_FENCED_BODY_CONTAINS: &str = "fenced-body-contains";

/// Parsed `inline-json-too-long` hint configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct InlineJsonConfig {
    /// Fence info strings this rule scopes to (e.g. `json`, `jsonc`).
    langs: Vec<String>,
    /// Maximum permitted fence-body line count.
    max_lines: u32,
}

impl InlineJsonConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::FencedBlock,
            reason: "`inline-json-too-long` requires a `config: { langs, max-lines }`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::FencedBlock,
            reason: "invalid fenced-block hint config JSON",
        })
    }
}

/// Parsed `fenced-body-contains` hint configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct BodyContainsConfig {
    /// Fence info strings this rule scopes to (e.g. `text`).
    langs: Vec<String>,
    /// Substrings whose presence in a matching fence body is a
    /// violation (e.g. the arrow glyphs of a text flow diagram).
    substrings: Vec<String>,
}

impl BodyContainsConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::FencedBlock,
            reason: "`fenced-body-contains` requires a `config: { langs, substrings }`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::FencedBlock,
            reason: "invalid fenced-block hint config JSON",
        })
    }
}

static ENVELOPE_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""envelope[-_]version"\s*:"#).expect("envelope version regex"));
static ENVELOPE_OK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""ok"\s*:\s*(true|false)\b"#).expect("envelope ok regex"));
static ENVELOPE_DATA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""data"\s*:"#).expect("envelope data regex"));
static ENVELOPE_ERROR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""error"\s*:\s*\{"#).expect("envelope error regex"));

fn is_envelope_body(body: &str) -> bool {
    if ENVELOPE_VERSION_RE.is_match(body) {
        return true;
    }
    let has_ok = ENVELOPE_OK_RE.is_match(body);
    let has_data = ENVELOPE_DATA_RE.is_match(body);
    let has_error = ENVELOPE_ERROR_RE.is_match(body);
    has_ok && (has_data || has_error)
}

fn is_json_fence(lang: &str) -> bool {
    lang == "json" || lang == "jsonc" || lang.starts_with("json ") || lang.starts_with("jsonc ")
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    match hint.value.trim() {
        SOURCE_SKILL_ENVELOPE_JSON_IN_BODY => Ok(envelope_json(rule, candidates, model, next_id)),
        SOURCE_INLINE_JSON_TOO_LONG => {
            let cfg = InlineJsonConfig::parse(rule, hint)?;
            Ok(inline_json_too_long(rule, candidates, model, &cfg, next_id))
        }
        SOURCE_FENCED_BODY_CONTAINS => {
            let cfg = BodyContainsConfig::parse(rule, hint)?;
            Ok(fenced_body_contains(rule, candidates, model, &cfg, next_id))
        }
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::FencedBlock,
            reason: "unknown fenced-block source discriminator",
        }),
    }
}

fn envelope_json(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);
    let mut findings = Vec::new();

    for block in &model.fenced_blocks {
        if !candidate_set.contains(&block.path) {
            continue;
        }
        if !is_json_fence(&block.lang) {
            continue;
        }
        if !is_envelope_body(&block.body) {
            continue;
        }
        findings.push(make_finding(
            rule,
            *next_id,
            format!(
                "Envelope JSON in skill body: {} — block at line {} (link to docs/reference/cli-output-shapes.md instead of embedding the envelope shape)",
                block.path, block.line_start
            ),
            Some(FindingLocation {
                path: block.path.clone(),
                line: Some(block.line_start),
                column: None,
                end_line: None,
                end_column: None,
            }),
            FindingEvidence::Structured {
                summary: "envelope-json-in-body".to_string(),
                data: serde_json::json!({ "line-start": block.line_start }),
                locations: None,
            },
        ));
        *next_id += 1;
    }

    findings
}

/// Flag fences whose info string is one of `cfg.langs` and whose body
/// exceeds `cfg.max_lines`. The body line count is the fence's line span
/// (`line_end - line_start`), which counts every content line — blank or
/// not — exactly as the retired imperative predicate did.
fn inline_json_too_long(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel, cfg: &InlineJsonConfig,
    next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);
    let mut findings = Vec::new();

    for block in &model.fenced_blocks {
        if !candidate_set.contains(&block.path) {
            continue;
        }
        if !cfg.langs.iter().any(|lang| lang == &block.lang) {
            continue;
        }
        let body_lines = block.line_end.saturating_sub(block.line_start);
        if body_lines <= cfg.max_lines {
            continue;
        }
        findings.push(make_finding(
            rule,
            *next_id,
            format!(
                "Inline JSON too long: {}:{} — {} body lines (limit {}); move large output shapes to docs/reference/cli-output-shapes.md and link to them",
                block.path, block.line_start, body_lines, cfg.max_lines,
            ),
            Some(FindingLocation {
                path: block.path.clone(),
                line: Some(block.line_start),
                column: None,
                end_line: None,
                end_column: None,
            }),
            FindingEvidence::Structured {
                summary: format!("inline json fence has {body_lines} lines (limit {})", cfg.max_lines),
                data: serde_json::json!({
                    "path": block.path,
                    "line-start": block.line_start,
                    "actual": body_lines,
                    "max": cfg.max_lines,
                }),
                locations: None,
            },
        ));
        *next_id += 1;
    }

    findings
}

/// Flag fences whose info string is one of `cfg.langs` and whose body
/// contains any of `cfg.substrings`. One finding per matching fence.
fn fenced_body_contains(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel, cfg: &BodyContainsConfig,
    next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);
    let mut findings = Vec::new();

    for block in &model.fenced_blocks {
        if !candidate_set.contains(&block.path) {
            continue;
        }
        if !cfg.langs.iter().any(|lang| lang == &block.lang) {
            continue;
        }
        let Some(found) = cfg.substrings.iter().find(|needle| block.body.contains(needle.as_str()))
        else {
            continue;
        };
        findings.push(make_finding(
            rule,
            *next_id,
            format!(
                "Banned content in `{}` fence: {}:{} — body contains `{}`",
                block.lang, block.path, block.line_start, found,
            ),
            Some(FindingLocation {
                path: block.path.clone(),
                line: Some(block.line_start),
                column: None,
                end_line: None,
                end_column: None,
            }),
            FindingEvidence::Structured {
                summary: format!("fenced `{}` body contains `{found}`", block.lang),
                data: serde_json::json!({
                    "path": block.path,
                    "line-start": block.line_start,
                    "lang": block.lang,
                    "match": found,
                }),
                locations: None,
            },
        ));
        *next_id += 1;
    }

    findings
}

#[cfg(test)]
mod unit {
    use serde_json::json;

    use super::*;
    use crate::lint::FencedBlock;
    use crate::lint::eval::testkit::{candidates, empty_model, hint, hint_with_config, rule};

    fn block(path: &str, lang: &str, body: &str, line_start: u32, line_end: u32) -> FencedBlock {
        FencedBlock {
            path: path.to_string(),
            line_start,
            line_end,
            lang: lang.to_string(),
            body: body.to_string(),
        }
    }

    #[test]
    fn envelope_shapes_detected() {
        let mut model = empty_model();
        let path = "plugins/p/skills/s/SKILL.md";
        model.fenced_blocks = vec![
            block(path, "json", r#"{ "envelope-version": 6 }"#, 3, 5),
            block(path, "json", r#"{ "ok": true, "data": {} }"#, 8, 10),
            block(path, "json", r#"{ "ok": true }"#, 12, 13),
            block(path, "text", r#"{ "envelope-version": 6 }"#, 15, 17),
        ];
        let cands = candidates(&[path]);
        let hint = hint(HintKind::FencedBlock, "skill-envelope-json-in-body");
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        // The bare `ok` fence and the non-json fence stay silent.
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn long_json_fence_flagged() {
        let mut model = empty_model();
        let path = "docs/a.md";
        model.fenced_blocks = vec![
            block(path, "json", "{}", 1, 10),
            block(path, "json", "{}", 20, 22),
            block(path, "yaml", "{}", 30, 60),
        ];
        let cands = candidates(&[path]);
        let cfg = json!({ "langs": ["json"], "max-lines": 5 });
        let hint = hint_with_config(HintKind::FencedBlock, "inline-json-too-long", Some(cfg));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("docs/a.md:1"), "{}", out[0].title);
    }

    #[test]
    fn banned_substring_flagged() {
        let mut model = empty_model();
        let path = "docs/a.md";
        model.fenced_blocks =
            vec![block(path, "text", "a --> b", 1, 3), block(path, "text", "plain prose", 5, 7)];
        let cands = candidates(&[path]);
        let cfg = json!({ "langs": ["text"], "substrings": ["-->"] });
        let hint = hint_with_config(HintKind::FencedBlock, "fenced-body-contains", Some(cfg));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("`-->`"), "{}", out[0].title);
    }

    #[test]
    fn missing_config_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::FencedBlock, "inline-json-too-long");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn unknown_source_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::FencedBlock, "no-such-source");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
