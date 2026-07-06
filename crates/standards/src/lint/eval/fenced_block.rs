//! `kind: fenced-block` evaluator.
//!
//! Consumes [`crate::lint::FencedBlock`] facts from the indexer and
//! applies the closed `value` source discriminator:
//!
//! - `fenced-body-contains` (CORE-017) — flags fences whose info string
//!   is one of `config.langs` and whose body contains any of
//!   `config.substrings`. Both the language allow-list and the banned
//!   substring set are **policy supplied by the rule file**.

use std::path::PathBuf;

use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_FENCED_BODY_CONTAINS: &str = "fenced-body-contains";

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

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    match hint.value.trim() {
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

    // The CLI e2e defers per-`kind` eval semantics, so the source
    // discriminator collapses into one violation matrix and the
    // config-rejection arms into a second test.

    #[test]
    fn flags_fence_violations() {
        // `fenced-body-contains`: only the fence carrying a banned substring
        // fires; plain prose is silent.
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
    fn rejects_bad_config() {
        let model = empty_model();
        // A config-driven selector with no `config` is rejected.
        let hint = hint(HintKind::FencedBlock, "fenced-body-contains");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
        // An unknown source discriminator is rejected.
        let hint = hint_with_config(HintKind::FencedBlock, "no-such-source", None);
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
