//! `kind: constant-eq` evaluator.
//!
//! Asserts that some extracted field on a candidate fact matches an
//! expected value. One source discriminator ships today:
//!
//! - `skill-name-plugin-prefix` (CORE-043) — every well-formed skill
//!   `name` must begin with its owning plugin's discovery prefix
//!   (`<plugin>-`), modulo the per-plugin override map the rule supplies
//!   in `config: { overrides }`. The override map is **policy supplied
//!   by the rule file**, never a `const` in this arm (per the
//!   standards-layer policy-in-`specify` rule).
//!
//! Future hint values may extend the closed source set; unknown
//! discriminators are rejected as
//! [`super::HintError::Unsupported`] so authoring drift surfaces at
//! hint-evaluation time rather than silently passing.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_SKILL_NAME_PLUGIN_PREFIX: &str = "skill-name-plugin-prefix";

/// Well-formed kebab-case skill-name shape. A name that fails this
/// mechanism filter is left to the schema/grammar predicates; only a
/// well-formed name participates in the prefix check.
static SKILL_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]*$").expect("skill name regex"));

/// Parsed `skill-name-plugin-prefix` hint configuration. The override
/// map redirects a plugin directory to the discovery prefix its skill
/// names must carry (e.g. `spec -> specify`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct SkillNamePrefixConfig {
    /// `<plugin-dir> -> <required-prefix-base>` overrides.
    overrides: BTreeMap<String, String>,
}

impl SkillNamePrefixConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::ConstantEq,
            reason: "`skill-name-plugin-prefix` requires a `config: { overrides }`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::ConstantEq,
            reason: "invalid constant-eq hint config JSON",
        })
    }
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    match hint.value.trim() {
        SOURCE_SKILL_NAME_PLUGIN_PREFIX => {
            let cfg = SkillNamePrefixConfig::parse(rule, hint)?;
            Ok(skill_name_plugin_prefix(rule, candidates, model, &cfg, next_id))
        }
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::ConstantEq,
            reason: "unknown constant-eq source discriminator",
        }),
    }
}

/// Flag every candidate skill whose well-formed `name` does not begin
/// with its plugin's discovery prefix (`<plugin>-`, modulo the override
/// map). One finding per offending skill.
fn skill_name_plugin_prefix(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel,
    cfg: &SkillNamePrefixConfig, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);

    let mut out: Vec<Diagnostic> = Vec::new();
    for skill in &model.skills {
        if !candidate_set.contains(&skill.path) {
            continue;
        }
        if !SKILL_NAME_RE.is_match(&skill.name) {
            continue;
        }
        let base = cfg.overrides.get(&skill.plugin).map_or(skill.plugin.as_str(), String::as_str);
        let required_prefix = format!("{base}-");
        if skill.name.starts_with(&required_prefix) {
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
            summary: format!("skill '{}' name must start with '{}'", skill.name, required_prefix),
            data: serde_json::json!({
                "skill": skill.name,
                "path": skill.path,
                "plugin": skill.plugin,
                "required-prefix": required_prefix,
            }),
            locations: None,
        };
        let title = format!(
            "{}: skill name '{}' must start with '{}'",
            rule.title, skill.name, required_prefix,
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
    use crate::lint::Skill;
    use crate::lint::eval::testkit::{candidates, empty_model, hint_with_config, rule};

    fn skill(name: &str, plugin: &str) -> Skill {
        Skill {
            name: name.to_string(),
            path: format!("plugins/{plugin}/skills/{name}/SKILL.md"),
            plugin: plugin.to_string(),
            frontmatter_ref: String::new(),
            body_line_count: None,
        }
    }

    // The CLI e2e defers per-`kind` eval semantics, so the source
    // discriminator collapses into one divergence matrix and the
    // config-rejection arms into a second test.

    #[test]
    fn flags_constant_divergences() {
        // `skill-name-plugin-prefix`: only the override-violating name fires;
        // the overridden, un-overridden, and malformed names are silent.
        let mut model = empty_model();
        model.skills = vec![
            skill("spec-plan", "spec"),    // wrong: spec maps to specify-
            skill("specify-plan", "spec"), // right under the override
            skill("client-sow", "client"), // right without an override
            skill("Bad Name", "client"),   // malformed: skipped by the shape filter
        ];
        let cands = candidates(&[
            "plugins/spec/skills/spec-plan/SKILL.md",
            "plugins/spec/skills/specify-plan/SKILL.md",
            "plugins/client/skills/client-sow/SKILL.md",
            "plugins/client/skills/Bad Name/SKILL.md",
        ]);
        let cfg = json!({ "overrides": { "spec": "specify" } });
        let hint = hint_with_config(HintKind::ConstantEq, "skill-name-plugin-prefix", Some(cfg));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'spec-plan'"), "{}", out[0].title);
    }

    #[test]
    fn rejects_bad_config() {
        let model = empty_model();
        // A selector with no `config` is rejected.
        let hint = hint_with_config(HintKind::ConstantEq, "skill-name-plugin-prefix", None);
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
        // An unknown source discriminator is rejected.
        let hint = hint_with_config(HintKind::ConstantEq, "no-such-source", Some(json!({})));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
