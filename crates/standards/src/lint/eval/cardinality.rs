//! `kind: cardinality` evaluator.
//!
//! Asserts that some countable property of a candidate fact is within a
//! configured bound. The `value` is a pure *metric selector* naming
//! which fact family and field to count; the numeric cap is **policy
//! supplied by the rule's `config: { max }`**, never a `const` in this
//! arm (per the standards-layer policy-in-`specify` rule).
//!
//! Metric selectors that ship today:
//!
//! - `markdown-h2-section-body-line-count` — counts the body lines of
//!   every level-2 [`crate::lint::MarkdownSection`] in the candidate set
//!   and flags each section whose `body_line_count` exceeds
//!   `config.max`. One finding per over-budget section, located at the
//!   heading line, with the `(actual, max)` pair surfaced via
//!   [`specify_diagnostics::FindingEvidence::Structured`].
//! - `skill-body-line-count` — counts the body lines of every
//!   [`crate::lint::Skill`] in the candidate set (CORE-005) and flags
//!   each skill whose `body_line_count` exceeds `config.max`. The cap
//!   is policy supplied by the rule, never a `const` in this arm.
//!
//! Facts whose `path` is not in the caller-supplied candidate set are
//! ignored, so the closed `path-pattern` filter the umbrella evaluator
//! builds still drives candidate selection. Unknown metric selectors are
//! rejected as [`super::HintError::Unsupported`] so authoring drift
//! surfaces at hint-evaluation time rather than silently passing.

use std::path::PathBuf;

use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

/// Config-driven metric: whole-skill body-line cap (CORE-005). The
/// cap is read from `config.max`.
const SOURCE_SKILL_BODY_LINE_COUNT: &str = "skill-body-line-count";

/// Config-driven metric: per-section H2 body-line cap (CORE-045). The
/// cap is read from `config.max`.
const SOURCE_MARKDOWN_H2_SECTION_BODY_LINE_COUNT: &str = "markdown-h2-section-body-line-count";

/// Parsed `cardinality` hint configuration. `max` is the upper bound
/// the rule file supplies; this arm never embeds a numeric cap for a
/// config-driven metric.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct CardinalityConfig {
    max: u32,
}

impl CardinalityConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Cardinality,
            reason: "this cardinality metric requires a `config: { max }`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Cardinality,
            reason: "invalid cardinality hint config JSON",
        })
    }
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    match hint.value.trim() {
        SOURCE_SKILL_BODY_LINE_COUNT => {
            let cfg = CardinalityConfig::parse(rule, hint)?;
            Ok(skill_body(rule, candidates, model, cfg.max, next_id))
        }
        SOURCE_MARKDOWN_H2_SECTION_BODY_LINE_COUNT => {
            let cfg = CardinalityConfig::parse(rule, hint)?;
            Ok(markdown_h2_sections(rule, candidates, model, cfg.max, next_id))
        }
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Cardinality,
            reason: "unknown cardinality metric selector",
        }),
    }
}

/// Config-driven CORE-005 whole-skill body-line cap. The cap is the
/// `max` the rule file supplies.
fn skill_body(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel, max: u32,
    next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);

    let mut out: Vec<Diagnostic> = Vec::new();
    for skill in &model.skills {
        if !candidate_set.contains(&skill.path) {
            continue;
        }
        let Some(actual) = skill.body_line_count else { continue };
        if actual <= max {
            continue;
        }
        let location = FindingLocation {
            path: skill.path.clone(),
            line: Some(1),
            column: None,
            end_line: None,
            end_column: None,
        };
        let evidence = FindingEvidence::Structured {
            summary: format!("skill '{}' body has {} lines (limit {})", skill.name, actual, max),
            data: serde_json::json!({
                "skill": skill.name,
                "path": skill.path,
                "actual": actual,
                "max": max,
            }),
            locations: None,
        };
        let title = format!(
            "{}: skill '{}' body exceeds the {}-line cap ({} lines)",
            rule.title, skill.name, max, actual,
        );
        let finding = make_finding(rule, *next_id, title, Some(location), evidence);
        *next_id += 1;
        out.push(finding);
    }
    out
}

/// Config-driven CORE-045 per-section H2 body-line cap. The cap is the
/// `max` the rule file supplies.
fn markdown_h2_sections(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel, max: u32,
    next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);

    let mut out: Vec<Diagnostic> = Vec::new();
    for section in &model.markdown_sections {
        if section.level != 2 {
            continue;
        }
        if !candidate_set.contains(&section.path) {
            continue;
        }
        if section.body_line_count <= max {
            continue;
        }
        let location = FindingLocation {
            path: section.path.clone(),
            line: Some(section.line_start),
            column: None,
            end_line: None,
            end_column: None,
        };
        let evidence = FindingEvidence::Structured {
            summary: format!(
                "section '{}' has {} lines (limit {})",
                section.title, section.body_line_count, max,
            ),
            data: serde_json::json!({
                "title": section.title,
                "path": section.path,
                "actual": section.body_line_count,
                "max": max,
            }),
            locations: None,
        };
        let title = format!(
            "{}: section '{}' exceeds the {}-line cap ({} lines)",
            rule.title, section.title, max, section.body_line_count,
        );
        let finding = make_finding(rule, *next_id, title, Some(location), evidence);
        *next_id += 1;
        out.push(finding);
    }
    out
}

#[cfg(test)]
mod unit {
    use serde_json::json;

    use super::*;
    use crate::lint::eval::testkit::{candidates, empty_model, hint, hint_with_config, rule};
    use crate::lint::{MarkdownSection, Skill};

    fn skill(name: &str, path: &str, lines: u32) -> Skill {
        Skill {
            name: name.to_string(),
            path: path.to_string(),
            plugin: "p".to_string(),
            frontmatter_ref: path.to_string(),
            body_line_count: Some(lines),
        }
    }

    fn section(path: &str, level: u8, title: &str, lines: u32) -> MarkdownSection {
        MarkdownSection {
            path: path.to_string(),
            level,
            title: title.to_string(),
            line_start: 1,
            line_end: 1 + lines,
            body_line_count: lines,
        }
    }

    fn flagged_paths(out: &[Diagnostic]) -> Vec<String> {
        out.iter().filter_map(|f| Some(f.location.as_ref()?.path.clone())).collect()
    }

    // The CLI e2e defers per-`kind` eval semantics, so the metric
    // selectors collapse into one over-cap matrix and the config-rejection
    // arms into a second test. Every former input is preserved.

    #[test]
    fn metrics_flag_over_cap() {
        // `skill-body-line-count`: an over-cap skill in the candidate set is
        // flagged; a candidate-set miss is silent.
        let mut model = empty_model();
        model.skills = vec![
            skill("big", "plugins/p/skills/big/SKILL.md", 6),
            skill("small", "plugins/p/skills/small/SKILL.md", 1),
        ];
        let cfg = Some(json!({ "max": 3 }));
        let cands =
            candidates(&["plugins/p/skills/big/SKILL.md", "plugins/p/skills/small/SKILL.md"]);
        let hint = hint_with_config(HintKind::Cardinality, "skill-body-line-count", cfg.clone());
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(flagged_paths(&out), vec!["plugins/p/skills/big/SKILL.md"]);
        // The same over-cap skill outside the candidate set is skipped.
        let hint = hint_with_config(HintKind::Cardinality, "skill-body-line-count", cfg);
        assert!(evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate").is_empty());

        // `markdown-h2-section-body-line-count`: only the over-cap level-2
        // section fires; an over-cap H3 is out of scope.
        let mut model = empty_model();
        let path = "plugins/p/skills/s/SKILL.md";
        model.markdown_sections = vec![
            section(path, 2, "Big", 4),
            section(path, 2, "Small", 1),
            section(path, 3, "Deep", 9),
        ];
        let cands = candidates(&[path]);
        let hint = hint_with_config(
            HintKind::Cardinality,
            "markdown-h2-section-body-line-count",
            Some(json!({ "max": 3 })),
        );
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'Big'"), "{}", out[0].title);
    }

    #[test]
    fn rejects_bad_config() {
        let model = empty_model();
        // A metric with no `config: { max }` is rejected.
        let hint = hint(HintKind::Cardinality, "markdown-h2-section-body-line-count");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
        // An unknown metric selector is rejected.
        let hint =
            hint_with_config(HintKind::Cardinality, "no-such-metric", Some(json!({ "max": 1 })));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
