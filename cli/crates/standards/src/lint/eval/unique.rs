//! `kind: unique` evaluator.
//!
//! Asserts that some fact field is unique across a fact family. v1
//! supports two fact-family selectors:
//!
//! - `skill` — over the [`crate::lint::Skill`] facts the
//!   framework-profile indexer produced (see
//!   [`crate::lint::index::skill`]); the only supported field is
//!   `skill-name`, narrowed by the `path-pattern` candidate set.
//! - `scenario` — over the [`crate::lint::Scenario`] facts the
//!   scenario pass produced (see [`crate::lint::index::scenario`]); the
//!   only supported field is `id`, evaluated whole-tree (scenario files
//!   are kept out of `model.files`, so the candidate set can never
//!   select them).
//!
//! The field to enforce uniqueness on is **the field selector the rule
//! supplies in `config: { field }`**, not a `const` discriminator in
//! this arm. The interpreter emits one
//! [`specify_diagnostics::Diagnostic`] per duplicated value, with the lowest
//! offending path used as the finding's location and the full sorted
//! path list surfaced via [`specify_diagnostics::FindingEvidence::Structured`].
//!
//! Facts missing the selected field are dropped upstream and never
//! reach this layer.
//!
//! Future hint values may extend the closed selector / field sets;
//! unknown selectors and fields are rejected as
//! [`super::HintError::Unsupported`] so authoring drift surfaces at
//! hint-evaluation time rather than silently passing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_SKILL: &str = "skill";
const SOURCE_SCENARIO: &str = "scenario";
/// The fields each fact family can enforce uniqueness on today; naming
/// a fact field is mechanism.
const FIELD_SKILL_NAME: &str = "skill-name";
const FIELD_SCENARIO_ID: &str = "id";

/// Parsed `unique` hint configuration. The field selector is supplied
/// by the rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct UniqueConfig {
    field: String,
}

impl UniqueConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Unique,
            reason: "`unique` requires a `config: { field }`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Unique,
            reason: "invalid unique hint config JSON",
        })
    }
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    let cfg = UniqueConfig::parse(rule, hint)?;
    match hint.value.trim() {
        SOURCE_SKILL => evaluate_skill(rule, &cfg, candidates, model, next_id),
        SOURCE_SCENARIO => evaluate_scenario(rule, &cfg, model, next_id),
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Unique,
            reason: "only `skill` or `scenario` is supported in v1",
        }),
    }
}

/// `skill` selector: each `name:` value across the candidate-narrowed
/// `plugins/**/SKILL.md` set must be unique.
fn evaluate_skill(
    rule: &ResolvedRule, cfg: &UniqueConfig, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    if cfg.field != FIELD_SKILL_NAME {
        return Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Unique,
            reason: "only the `skill-name` field is supported for `skill` in v1",
        });
    }
    let candidate_set = super::candidate_set(candidates);
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for skill in &model.skills {
        if !candidate_set.contains(&skill.path) {
            continue;
        }
        groups.entry(skill.name.clone()).or_default().insert(skill.path.clone());
    }
    Ok(emit_duplicates(rule, &groups, "name", "skill name", next_id))
}

/// `scenario` selector: each frontmatter `id` across the whole scenario
/// fact family must be unique. Whole-tree (no candidate narrowing).
fn evaluate_scenario(
    rule: &ResolvedRule, cfg: &UniqueConfig, model: &WorkspaceModel, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    if cfg.field != FIELD_SCENARIO_ID {
        return Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::Unique,
            reason: "only the `id` field is supported for `scenario` in v1",
        });
    }
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for scenario in &model.scenarios {
        let Some(id) = scenario.id.as_ref() else {
            continue;
        };
        groups.entry(id.clone()).or_default().insert(scenario.path.clone());
    }
    Ok(emit_duplicates(rule, &groups, "id", "scenario id", next_id))
}

/// Emit one finding per duplicated value (group of two or more paths),
/// keyed under `value_key` in the structured evidence with `noun` in
/// the prose. The lowest offending path is the finding location.
fn emit_duplicates(
    rule: &ResolvedRule, groups: &BTreeMap<String, BTreeSet<String>>, value_key: &str, noun: &str,
    next_id: &mut u64,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    for (value, paths) in groups {
        if paths.len() < 2 {
            continue;
        }
        let sorted: Vec<String> = paths.iter().cloned().collect();
        let Some(first) = sorted.first().cloned() else {
            continue;
        };
        let location = FindingLocation {
            path: first,
            line: Some(1),
            column: None,
            end_line: None,
            end_column: None,
        };
        let locations: Vec<FindingLocation> = sorted
            .iter()
            .map(|p| FindingLocation {
                path: p.clone(),
                line: Some(1),
                column: None,
                end_line: None,
                end_column: None,
            })
            .collect();
        let mut data = serde_json::Map::new();
        data.insert(value_key.to_owned(), serde_json::Value::String(value.clone()));
        data.insert("paths".to_owned(), serde_json::json!(sorted));
        let evidence = FindingEvidence::Structured {
            summary: format!("duplicate {noun} '{value}'"),
            data: serde_json::Value::Object(data),
            locations: Some(locations),
        };
        let title = format!("{}: duplicate {noun} '{value}'", rule.title);
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
    use crate::lint::{Scenario, Skill};

    fn skill(name: &str, path: &str) -> Skill {
        Skill {
            name: name.to_string(),
            path: path.to_string(),
            plugin: "p".to_string(),
            frontmatter_ref: path.to_string(),
            body_line_count: None,
        }
    }

    fn scenario(id: Option<&str>, path: &str) -> Scenario {
        Scenario {
            path: path.to_string(),
            id: id.map(str::to_string),
            stages: vec![],
            expected_artifacts: vec![],
            body_id: None,
            fields: serde_json::Map::new(),
        }
    }

    #[test]
    fn duplicate_skill_names_flagged() {
        let mut model = empty_model();
        model.skills = vec![
            skill("dup", "plugins/a/skills/x/SKILL.md"),
            skill("dup", "plugins/b/skills/y/SKILL.md"),
            skill("solo", "plugins/c/skills/z/SKILL.md"),
        ];
        let cands = candidates(&[
            "plugins/a/skills/x/SKILL.md",
            "plugins/b/skills/y/SKILL.md",
            "plugins/c/skills/z/SKILL.md",
        ]);
        let hint =
            hint_with_config(HintKind::Unique, "skill", Some(json!({ "field": "skill-name" })));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'dup'"), "{}", out[0].title);
    }

    #[test]
    fn duplicate_outside_candidates_skipped() {
        let mut model = empty_model();
        model.skills = vec![
            skill("dup", "plugins/a/skills/x/SKILL.md"),
            skill("dup", "plugins/b/skills/y/SKILL.md"),
        ];
        let cands = candidates(&["plugins/a/skills/x/SKILL.md"]);
        let hint =
            hint_with_config(HintKind::Unique, "skill", Some(json!({ "field": "skill-name" })));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert!(out.is_empty());
    }

    #[test]
    fn duplicate_scenario_ids_flagged_whole_tree() {
        let mut model = empty_model();
        model.scenarios = vec![
            scenario(Some("s-1"), "evals/scenarios/a.md"),
            scenario(Some("s-1"), "evals/scenarios/b.md"),
            scenario(None, "evals/scenarios/c.md"),
        ];
        let hint = hint_with_config(HintKind::Unique, "scenario", Some(json!({ "field": "id" })));
        let out = evaluate(&rule(), &hint, &[], &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'s-1'"), "{}", out[0].title);
    }

    #[test]
    fn wrong_field_is_unsupported() {
        let model = empty_model();
        let hint = hint_with_config(HintKind::Unique, "skill", Some(json!({ "field": "id" })));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn missing_config_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::Unique, "skill");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn unknown_selector_is_unsupported() {
        let model = empty_model();
        let hint = hint_with_config(HintKind::Unique, "adapter", Some(json!({ "field": "name" })));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
