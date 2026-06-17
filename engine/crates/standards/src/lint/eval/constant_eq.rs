//! `kind: constant-eq` evaluator.
//!
//! Asserts that some extracted field on a candidate fact matches an
//! expected value — either a fixed constant or a value derived from a
//! sibling field. Two source discriminators ship today:
//!
//! - `skill-name-plugin-prefix` (CORE-043) — every well-formed skill
//!   `name` must begin with its owning plugin's discovery prefix
//!   (`<plugin>-`), modulo the per-plugin override map the rule supplies
//!   in `config: { overrides }`. The override map is **policy supplied
//!   by the rule file**, never a `const` in this arm (per the
//!   standards-layer policy-in-`specify` rule).
//! - `adapter-manifest-field` (CORE-006) — assert that a named field on
//!   every [`crate::lint::AdapterManifest`] in the candidate set equals
//!   a fixed value. Both the field selector and the expected value are
//!   **policy supplied by the rule's `config: { field, equals }`**,
//!   never a `const` in this arm (per the standards-layer
//!   policy-in-`specify` rule). v1 understands the `version` field only.
//!
//! `adapter-manifest-field` consumes the
//! [`crate::lint::AdapterManifest`] facts the framework-profile
//! indexer already produced
//! (see [`crate::lint::index::adapter::extract`], whose `version`
//! field stringifies both integer and string YAML forms) and flags
//! each `adapters/{sources,targets}/<name>/adapter.yaml` whose
//! selected field does not equal `config.equals`. The
//! interpreter emits one [`specify_diagnostics::Diagnostic`] per
//! non-conforming manifest with the manifest path as the finding's
//! location and the `(actual, expected)` pair surfaced via
//! [`specify_diagnostics::FindingEvidence::Structured`] for downstream
//! tooling. Manifests whose `version:` is absent count as actual
//! `"(absent)"`; that string can never collide with a real version
//! because the extractor rejects empty / non-string-or-number
//! values up front.
//!
//! Adapter manifests whose `path` is not in the caller-supplied
//! candidate set are ignored, so the closed `path-pattern` filter
//! the umbrella evaluator builds still drives candidate selection.
//! Manifests the indexer drops upstream (binary `adapter.yaml`,
//! YAML body without a non-empty `name:` value, etc.) never reach
//! this layer.
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

const SOURCE_ADAPTER_MANIFEST_FIELD: &str = "adapter-manifest-field";
/// The only adapter-manifest field this arm can read today; naming a
/// fact attribute is mechanism, the value it must equal is policy.
const FIELD_VERSION: &str = "version";
const ABSENT_VERSION_TOKEN: &str = "(absent)";

const SOURCE_SKILL_NAME_PLUGIN_PREFIX: &str = "skill-name-plugin-prefix";

/// Parsed `adapter-manifest-field` hint configuration. Both the field
/// selector and the expected value are policy supplied by the rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ConstantFieldEqConfig {
    field: String,
    equals: String,
}

impl ConstantFieldEqConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::ConstantEq,
            reason: "`adapter-manifest-field` requires a `config: { field, equals }`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::ConstantEq,
            reason: "invalid constant-eq hint config JSON",
        })
    }
}

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
        SOURCE_ADAPTER_MANIFEST_FIELD => {
            let cfg = ConstantFieldEqConfig::parse(rule, hint)?;
            if cfg.field != FIELD_VERSION {
                return Err(HintError::Unsupported {
                    rule_id: rule.rule_id.clone(),
                    kind: HintKind::ConstantEq,
                    reason: "only the `version` adapter-manifest field is supported in v1",
                });
            }
            Ok(adapter_manifest_field(rule, candidates, model, &cfg, next_id))
        }
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

/// Flag every candidate adapter manifest whose `version` field does not
/// equal `config.equals`. The expected value is the policy the rule
/// file supplies.
fn adapter_manifest_field(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel,
    cfg: &ConstantFieldEqConfig, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);

    let mut out: Vec<Diagnostic> = Vec::new();
    for manifest in &model.adapter_manifests {
        if !candidate_set.contains(&manifest.path) {
            continue;
        }
        let actual = manifest.version.as_deref().unwrap_or(ABSENT_VERSION_TOKEN);
        if actual == cfg.equals {
            continue;
        }
        let location = FindingLocation {
            path: manifest.path.clone(),
            line: Some(1),
            column: None,
            end_line: None,
            end_column: None,
        };
        let evidence = FindingEvidence::Structured {
            summary: format!(
                "adapter '{}' declares version '{}' (expected '{}')",
                manifest.name, actual, cfg.equals,
            ),
            data: serde_json::json!({
                "adapter": manifest.name,
                "path": manifest.path,
                "field": cfg.field,
                "actual": actual,
                "expected": cfg.equals,
            }),
            locations: None,
        };
        let title = format!(
            "{}: adapter '{}' version '{}' does not equal '{}'",
            rule.title, manifest.name, actual, cfg.equals,
        );
        let finding = make_finding(rule, *next_id, title, Some(location), evidence);
        *next_id += 1;
        out.push(finding);
    }
    out
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
    use crate::lint::eval::testkit::{candidates, empty_model, hint, hint_with_config, rule};
    use crate::lint::{AdapterAxis, AdapterManifest, Skill};

    fn manifest(name: &str, version: Option<&str>) -> AdapterManifest {
        AdapterManifest {
            axis: AdapterAxis::Targets,
            name: name.to_string(),
            path: format!("adapters/targets/{name}/adapter.yaml"),
            version: version.map(str::to_string),
            brief_keys: vec![],
        }
    }

    fn skill(name: &str, plugin: &str) -> Skill {
        Skill {
            name: name.to_string(),
            path: format!("plugins/{plugin}/skills/{name}/SKILL.md"),
            plugin: plugin.to_string(),
            frontmatter_ref: String::new(),
            body_line_count: None,
        }
    }

    #[test]
    fn version_mismatch_and_absence_flagged() {
        let mut model = empty_model();
        model.adapter_manifests =
            vec![manifest("ok", Some("2")), manifest("old", Some("1")), manifest("none", None)];
        let cands = candidates(&[
            "adapters/targets/ok/adapter.yaml",
            "adapters/targets/old/adapter.yaml",
            "adapters/targets/none/adapter.yaml",
        ]);
        let cfg = json!({ "field": "version", "equals": "2" });
        let hint = hint_with_config(HintKind::ConstantEq, "adapter-manifest-field", Some(cfg));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        let titles: Vec<&str> = out.iter().map(|f| f.title.as_str()).collect();
        assert_eq!(out.len(), 2, "{titles:?}");
        assert!(titles.iter().any(|t| t.contains("'old' version '1'")), "{titles:?}");
        assert!(titles.iter().any(|t| t.contains("'none' version '(absent)'")), "{titles:?}");
    }

    #[test]
    fn non_version_field_is_unsupported() {
        let model = empty_model();
        let cfg = json!({ "field": "name", "equals": "x" });
        let hint = hint_with_config(HintKind::ConstantEq, "adapter-manifest-field", Some(cfg));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn prefix_violations_flagged_with_overrides() {
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
    fn missing_config_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::ConstantEq, "skill-name-plugin-prefix");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn unknown_source_is_unsupported() {
        let model = empty_model();
        let hint = hint_with_config(HintKind::ConstantEq, "no-such-source", Some(json!({})));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
