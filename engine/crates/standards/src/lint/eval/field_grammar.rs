//! `kind: field-grammar` evaluator.
//!
//! Flags a candidate whose named frontmatter `field` violates a
//! token / first-word grammar. `hint.value` selects one of two
//! mechanism modes, narrowed by the rule's `path-pattern` candidate
//! set over the [`WorkspaceModel::frontmatter`] fact family:
//!
//! - `field-tokens` — `config: { field, token-pattern }`; split the
//!   named `field` on whitespace and flag the candidate if any token
//!   fails the `token-pattern` regex. A present `field` whose value is
//!   not a string is flagged outright. For CORE-035.
//! - `field-first-word` — `config: { field, allowed }`; take the first
//!   alphabetic word of the named `field` and flag the candidate when
//!   it is not in the `allowed` list (or when no leading alphabetic
//!   word exists). A non-string `field` value is skipped. For CORE-036.
//!
//! All policy (the field name, the grammar regex, the allow-list) rides
//! the rule's `config:`; this arm names only mechanism — the two mode
//! tokens. An unknown mode, a missing required config field, or a
//! `token-pattern` regex that fails to compile are rejected as
//! [`super::HintError`] so authoring drift surfaces at hint-evaluation
//! time rather than silently passing.

use std::path::PathBuf;

use ::regex::Regex;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::{Frontmatter, WorkspaceModel};
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const VALUE_FIELD_TOKENS: &str = "field-tokens";
const VALUE_FIELD_FIRST_WORD: &str = "field-first-word";

/// Parsed `field-grammar` hint configuration. `field` is shared by both
/// modes; `token-pattern` and `allowed` are each required by exactly one
/// mode and validated there. The shape is schema-gated upstream by
/// `fieldGrammarHintConfig`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct FieldGrammarConfig {
    field: String,
    #[serde(default)]
    token_pattern: Option<String>,
    #[serde(default)]
    allowed: Option<Vec<String>>,
}

impl FieldGrammarConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::FieldGrammar,
            reason: "`field-grammar` requires a `config`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::FieldGrammar,
            reason: "invalid field-grammar hint config JSON",
        })
    }
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let cfg = FieldGrammarConfig::parse(rule, hint)?;
    match hint.value.trim() {
        VALUE_FIELD_TOKENS => evaluate_field_tokens(rule, &cfg, candidates, model, next_id),
        VALUE_FIELD_FIRST_WORD => evaluate_field_first_word(rule, &cfg, candidates, model, next_id),
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::FieldGrammar,
            reason: "only `field-tokens` or `field-first-word` is supported in v1",
        }),
    }
}

/// Candidate frontmatter facts narrowed to the rule's `path-pattern`
/// set, in stable path order.
fn candidate_frontmatter<'model>(
    candidates: &[PathBuf], model: &'model WorkspaceModel,
) -> Vec<&'model Frontmatter> {
    let candidate_set = super::candidate_set(candidates);
    let mut out: Vec<&Frontmatter> =
        model.frontmatter.iter().filter(|fm| candidate_set.contains(&fm.path)).collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// `field-tokens` mode: every whitespace-separated token of the named
/// `field` must match the `token-pattern` regex; a present `field` that
/// is not a string is flagged outright.
fn evaluate_field_tokens(
    rule: &ResolvedRule, cfg: &FieldGrammarConfig, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let pattern = cfg.token_pattern.as_deref().ok_or_else(|| HintError::Unsupported {
        rule_id: rule.rule_id.clone(),
        kind: HintKind::FieldGrammar,
        reason: "`field-tokens` requires a `config: { token-pattern }`",
    })?;
    let token_re = Regex::new(pattern).map_err(|err| HintError::RegexCompile {
        rule_id: rule.rule_id.clone(),
        pattern: pattern.to_owned(),
        source: err,
    })?;
    let mut out: Vec<Diagnostic> = Vec::new();
    for fm in candidate_frontmatter(candidates, model) {
        let Some(value) = fm.fields.get(&cfg.field) else {
            continue;
        };
        let Some(text) = value.as_str() else {
            let summary =
                format!("frontmatter field '{}' must be a string in '{}'", cfg.field, fm.path);
            out.push(mint(rule, &fm.path, &summary, next_id));
            continue;
        };
        if let Some(token) = first_invalid_token(text, &token_re) {
            let summary = format!(
                "frontmatter field '{}' token '{token}' (in '{text}') does not match the required grammar",
                cfg.field
            );
            out.push(mint(rule, &fm.path, &summary, next_id));
        }
    }
    Ok(out)
}

/// `field-first-word` mode: the first alphabetic word of the named
/// `field` must be a member of the `allowed` list. A non-string `field`
/// value is skipped; a string with no leading alphabetic word is
/// flagged.
fn evaluate_field_first_word(
    rule: &ResolvedRule, cfg: &FieldGrammarConfig, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let allowed = cfg.allowed.as_deref().ok_or_else(|| HintError::Unsupported {
        rule_id: rule.rule_id.clone(),
        kind: HintKind::FieldGrammar,
        reason: "`field-first-word` requires a `config: { allowed }`",
    })?;
    let mut out: Vec<Diagnostic> = Vec::new();
    for fm in candidate_frontmatter(candidates, model) {
        let Some(text) = fm.fields.get(&cfg.field).and_then(JsonValue::as_str) else {
            continue;
        };
        let first_word = text.split_whitespace().next().unwrap_or("");
        let first_alpha: String =
            first_word.chars().take_while(char::is_ascii_alphabetic).collect();
        if first_alpha.is_empty() {
            let summary = format!(
                "frontmatter field '{}' must start with a word in the allow-list in '{}' — no leading word found",
                cfg.field, fm.path
            );
            out.push(mint(rule, &fm.path, &summary, next_id));
            continue;
        }
        let lower = first_alpha.to_ascii_lowercase();
        if allowed.iter().any(|verb| verb == &lower) {
            continue;
        }
        let summary = format!(
            "frontmatter field '{}' first word '{first_alpha}' is not in the allow-list in '{}'",
            cfg.field, fm.path
        );
        out.push(mint(rule, &fm.path, &summary, next_id));
    }
    Ok(out)
}

/// First whitespace-separated token of `text` that fails `token_re`, or
/// `None` when every token matches (an empty value passes vacuously).
fn first_invalid_token(text: &str, token_re: &Regex) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.split_whitespace().find(|token| !token_re.is_match(token)).map(str::to_owned)
}

/// Mint one field-grammar finding located at `path`, with structured
/// evidence carrying the offending path, and bump the id counter.
fn mint(rule: &ResolvedRule, path: &str, summary: &str, next_id: &mut u64) -> Diagnostic {
    let location = FindingLocation {
        path: path.to_owned(),
        line: None,
        column: None,
        end_line: None,
        end_column: None,
    };
    let evidence = FindingEvidence::Structured {
        summary: summary.to_owned(),
        data: serde_json::json!({ "path": path }),
        locations: None,
    };
    let title = format!("{}: {summary}", rule.title);
    let finding = make_finding(rule, *next_id, title, Some(location), evidence);
    *next_id += 1;
    finding
}

#[cfg(test)]
mod unit {
    use serde_json::json;

    use super::*;
    use crate::lint::eval::testkit::{candidates, empty_model, hint, hint_with_config, rule};

    fn frontmatter(path: &str, field: &str, value: serde_json::Value) -> Frontmatter {
        let mut fields = serde_json::Map::new();
        fields.insert(field.to_string(), value);
        Frontmatter {
            path: path.to_string(),
            schema_id: None,
            fields,
        }
    }

    #[test]
    fn tokens_flag_bad_and_pass_good() {
        let mut model = empty_model();
        model.frontmatter = vec![
            frontmatter("good.md", "argument-hint", json!("<slice-dir> [crate-name]")),
            frontmatter("bad.md", "argument-hint", json!("the slice name")),
            frontmatter("non-string.md", "argument-hint", json!(7)),
        ];
        let cands = candidates(&["good.md", "bad.md", "non-string.md"]);
        let cfg =
            json!({ "field": "argument-hint", "token-pattern": r"^[<\[][a-z][a-z0-9-]*[>\]]$" });
        let hint = hint_with_config(HintKind::FieldGrammar, "field-tokens", Some(cfg));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        let paths: Vec<&str> =
            out.iter().filter_map(|f| f.location.as_ref().map(|l| l.path.as_str())).collect();
        assert_eq!(paths, vec!["bad.md", "non-string.md"]);
    }

    #[test]
    fn first_word_allow_list() {
        let mut model = empty_model();
        model.frontmatter = vec![
            frontmatter("good.md", "description", json!("Build the demo fixtures.")),
            frontmatter("bad.md", "description", json!("The thing that does work.")),
            frontmatter("no-word.md", "description", json!("123 nope")),
        ];
        let cands = candidates(&["good.md", "bad.md", "no-word.md"]);
        let cfg = json!({ "field": "description", "allowed": ["build", "run"] });
        let hint = hint_with_config(HintKind::FieldGrammar, "field-first-word", Some(cfg));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        let paths: Vec<&str> =
            out.iter().filter_map(|f| f.location.as_ref().map(|l| l.path.as_str())).collect();
        assert_eq!(paths, vec!["bad.md", "no-word.md"]);
    }

    #[test]
    fn invalid_token_pattern_is_hard_error() {
        let model = empty_model();
        let cfg = json!({ "field": "x", "token-pattern": "(unclosed" });
        let hint = hint_with_config(HintKind::FieldGrammar, "field-tokens", Some(cfg));
        assert!(matches!(
            evaluate(&rule(), &hint, &[], &model, &mut 1),
            Err(HintError::RegexCompile { .. })
        ));
    }

    #[test]
    fn missing_config_and_unknown_mode_rejected() {
        let model = empty_model();
        let bare = hint(HintKind::FieldGrammar, "field-tokens");
        evaluate(&rule(), &bare, &[], &model, &mut 1).unwrap_err();
        let unknown =
            hint_with_config(HintKind::FieldGrammar, "no-such-mode", Some(json!({ "field": "x" })));
        evaluate(&rule(), &unknown, &[], &model, &mut 1).unwrap_err();
    }
}

#[cfg(test)]
mod tests {
    use ::regex::Regex;

    use super::first_invalid_token;

    // `first_invalid_token` is the per-token gate behind CORE-035. The
    // integration test only exercises the leading-token case; the subtle
    // contract is: an empty/whitespace value passes vacuously (returns
    // None), every-token-matches returns None, and when a *later* token
    // is the offender the scanner must return that later token, not the
    // first one it saw.
    #[test]
    fn vacuous_and_first_offender() {
        let re = Regex::new(r"^[<\[][a-z][a-z0-9-]*[>\]]$").expect("compile");

        assert_eq!(first_invalid_token("", &re), None, "empty value passes vacuously");
        assert_eq!(first_invalid_token("   \t ", &re), None, "whitespace-only passes vacuously");
        assert_eq!(first_invalid_token("<slice-dir> [crate]", &re), None, "all tokens conform");
        // The first token is fine; the second is prose — that second
        // token must be the one reported.
        assert_eq!(first_invalid_token("<slice-dir> the name", &re).as_deref(), Some("the"));
        assert_eq!(
            first_invalid_token("prose first", &re).as_deref(),
            Some("prose"),
            "a leading bad token is reported"
        );
    }
}
