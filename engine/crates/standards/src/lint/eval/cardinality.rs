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
//! - `brief-parent-body-line-count` / `brief-phase-body-line-count` —
//!   count the body lines of every [`crate::lint::Brief`] of the
//!   matching [`crate::lint::BriefScope`] and flag each brief whose
//!   `body_line_count` exceeds `config.max`. The two scopes carry
//!   distinct caps (a rule supplies both via `config`), so a rule that
//!   enforces both budgets ships two `cardinality` hints. The brief
//!   metrics narrow on a dedicated fact family already scoped to brief
//!   paths, so they do not consult the `path-pattern` candidate set.
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
use crate::lint::{BriefScope, WorkspaceModel};
use crate::rules::{HintKind, ResolvedRule, RuleHint};

/// Config-driven metric: whole-skill body-line cap (CORE-005). The
/// cap is read from `config.max`.
const SOURCE_SKILL_BODY_LINE_COUNT: &str = "skill-body-line-count";

/// Config-driven metric: per-section H2 body-line cap (CORE-045). The
/// cap is read from `config.max`.
const SOURCE_MARKDOWN_H2_SECTION_BODY_LINE_COUNT: &str = "markdown-h2-section-body-line-count";

/// Config-driven metric: parent-brief body-line cap (CORE-013). The
/// cap is read from `config.max`.
const SOURCE_BRIEF_PARENT_BODY_LINE_COUNT: &str = "brief-parent-body-line-count";

/// Config-driven metric: phase sub-brief body-line cap (CORE-013). The
/// cap is read from `config.max`.
const SOURCE_BRIEF_PHASE_BODY_LINE_COUNT: &str = "brief-phase-body-line-count";

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
        SOURCE_BRIEF_PARENT_BODY_LINE_COUNT => {
            let cfg = CardinalityConfig::parse(rule, hint)?;
            Ok(briefs(rule, model, BriefScope::Parent, cfg.max, next_id))
        }
        SOURCE_BRIEF_PHASE_BODY_LINE_COUNT => {
            let cfg = CardinalityConfig::parse(rule, hint)?;
            Ok(briefs(rule, model, BriefScope::Phase, cfg.max, next_id))
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

/// Config-driven CORE-013 per-brief body-line cap, scoped to one
/// [`BriefScope`]. The cap is the `max` the rule file supplies. The
/// `Brief` fact family is already restricted to brief paths, so this
/// metric evaluates it directly rather than narrowing by the
/// `path-pattern` candidate set.
fn briefs(
    rule: &ResolvedRule, model: &WorkspaceModel, scope: BriefScope, max: u32, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    for brief in &model.briefs {
        if brief.scope != scope {
            continue;
        }
        if brief.body_line_count <= max {
            continue;
        }
        let location = FindingLocation {
            path: brief.path.clone(),
            line: Some(1),
            column: None,
            end_line: None,
            end_column: None,
        };
        let evidence = FindingEvidence::Structured {
            summary: format!(
                "brief '{}' body has {} lines (limit {})",
                brief.path, brief.body_line_count, max,
            ),
            data: serde_json::json!({
                "path": brief.path,
                "actual": brief.body_line_count,
                "max": max,
            }),
            locations: None,
        };
        let title = format!(
            "{}: brief '{}' exceeds the {}-line cap ({} lines)",
            rule.title, brief.path, max, brief.body_line_count,
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
    use crate::lint::{Brief, MarkdownSection, Skill};

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

    fn brief(path: &str, scope: BriefScope, lines: u32) -> Brief {
        Brief {
            path: path.to_string(),
            axis: crate::lint::AdapterAxis::Targets,
            adapter: "demo".to_string(),
            operation: "build".to_string(),
            scope,
            sections: vec![],
            body_line_count: lines,
        }
    }

    fn flagged_paths(out: &[Diagnostic]) -> Vec<String> {
        out.iter().filter_map(|f| Some(f.location.as_ref()?.path.clone())).collect()
    }

    #[test]
    fn skill_body_over_cap() {
        let mut model = empty_model();
        model.skills = vec![
            skill("big", "plugins/p/skills/big/SKILL.md", 6),
            skill("small", "plugins/p/skills/small/SKILL.md", 1),
        ];
        let cands =
            candidates(&["plugins/p/skills/big/SKILL.md", "plugins/p/skills/small/SKILL.md"]);
        let hint = hint_with_config(
            HintKind::Cardinality,
            "skill-body-line-count",
            Some(json!({ "max": 3 })),
        );
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(flagged_paths(&out), vec!["plugins/p/skills/big/SKILL.md"]);
    }

    #[test]
    fn skill_outside_candidates_skipped() {
        let mut model = empty_model();
        model.skills = vec![skill("big", "plugins/p/skills/big/SKILL.md", 6)];
        let hint = hint_with_config(
            HintKind::Cardinality,
            "skill-body-line-count",
            Some(json!({ "max": 3 })),
        );
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        assert!(out.is_empty());
    }

    #[test]
    fn h2_sections_over_cap_only() {
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
        // Only the over-cap level-2 section fires; the over-cap H3 is out of scope.
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'Big'"), "{}", out[0].title);
    }

    #[test]
    fn brief_scopes_isolated() {
        let mut model = empty_model();
        model.briefs = vec![
            brief("adapters/targets/demo/briefs/build.md", BriefScope::Parent, 5),
            brief("adapters/targets/demo/briefs/build/phase.md", BriefScope::Phase, 2),
        ];
        let parent = hint_with_config(
            HintKind::Cardinality,
            "brief-parent-body-line-count",
            Some(json!({ "max": 3 })),
        );
        let out = evaluate(&rule(), &parent, &[], &model, &mut 1).expect("evaluate");
        assert_eq!(flagged_paths(&out), vec!["adapters/targets/demo/briefs/build.md"]);

        // A phase cap the phase brief clears but the parent would exceed:
        // scope isolation keeps the parent silent under the phase metric.
        let phase = hint_with_config(
            HintKind::Cardinality,
            "brief-phase-body-line-count",
            Some(json!({ "max": 1 })),
        );
        let out = evaluate(&rule(), &phase, &[], &model, &mut 1).expect("evaluate");
        assert_eq!(flagged_paths(&out), vec!["adapters/targets/demo/briefs/build/phase.md"]);
    }

    #[test]
    fn missing_config_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::Cardinality, "markdown-h2-section-body-line-count");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn unknown_metric_is_unsupported() {
        let model = empty_model();
        let hint =
            hint_with_config(HintKind::Cardinality, "no-such-metric", Some(json!({ "max": 1 })));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
