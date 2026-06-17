//! `kind: set-coverage` evaluator.
//!
//! Asserts that the set of values some candidate file declares
//! covers a closed expected set.
//!
//! `set-coverage` has two complementary directions, selected by the
//! `value` source discriminator:
//!
//! - `adapter-briefs` — an adapter's `briefs.keys()` checked against the
//!   axis-appropriate operation set the rule supplies in
//!   `config: { expected-operations }`. The `config: { mode }` selector
//!   chooses the direction: `subset` (the default) is the
//!   **expected ⊆ declared** check — only missing operations are
//!   flagged, extras are silent; `exact` is the two-sided **expected ==
//!   declared** check — missing operations *and* keys the manifest
//!   declares that are absent from the expected set are both flagged.
//!   The expected sets are **policy supplied by the rule file**, never a
//!   `const` in this arm (per the standards-layer policy-in-`specify`
//!   rule). It consumes the [`crate::lint::AdapterManifest`] facts the
//!   framework-profile indexer already produced
//!   (see [`crate::lint::index::adapter::extract`]) and emits one
//!   [`specify_diagnostics::Diagnostic`] per `(adapter, divergence)`
//!   pair, with the manifest path as the finding's location and the
//!   per-adapter divergence surfaced via
//!   [`specify_diagnostics::FindingEvidence::Structured`].
//! - `skill-allowed-tools` — the **declared ⊆ allowed** direction: every
//!   tool a skill lists in its `allowed-tools` frontmatter must be
//!   covered by the rule's `config: { allowed }` set (optionally with
//!   `allowed-prefixes` exemptions, e.g. `mcp__`); tools not covered are
//!   flagged. The recognised-tool set and prefix exemptions are
//!   **policy supplied by the rule file**, never a `const` in this arm
//!   (per the standards-layer policy-in-`specify` rule).
//!
//! Facts whose `path` is not in the caller-supplied candidate set are
//! ignored, so the closed `path-pattern` filter the umbrella evaluator
//! builds still drives candidate selection. Unknown discriminators are
//! rejected as [`super::HintError::Unsupported`] so authoring drift
//! surfaces at hint-evaluation time rather than silently passing.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value as JsonValue;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::lint::adapter_briefs::{BriefsMode, ExpectedOperationsConfig, axis_token};
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_ADAPTER_BRIEFS: &str = "adapter-briefs";
const SOURCE_SKILL_ALLOWED_TOOLS: &str = "skill-allowed-tools";

/// Divergence direction for an operation that breaks set equality
/// (surfaced under `mode: exact`).
const DIVERGENCE_MISSING: &str = "missing";
const DIVERGENCE_UNEXPECTED: &str = "unexpected";

/// Parsed `skill-allowed-tools` hint configuration. Both the recognised
/// tool set and the prefix exemptions are policy supplied by the rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct AllowedToolsConfig {
    /// The closed set of recognised tool names.
    allowed: Vec<String>,
    /// Prefixes that exempt a tool from the allow-list (e.g. `mcp__`
    /// for dynamically-named MCP tools).
    #[serde(default)]
    allowed_prefixes: Vec<String>,
}

impl AllowedToolsConfig {
    fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let raw = hint.config.as_ref().ok_or_else(|| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::SetCoverage,
            reason: "`skill-allowed-tools` requires a `config: { allowed }`",
        })?;
        serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::SetCoverage,
            reason: "invalid set-coverage hint config JSON",
        })
    }

    fn covers(&self, tool: &str) -> bool {
        self.allowed.iter().any(|allowed| allowed == tool)
            || self.allowed_prefixes.iter().any(|prefix| tool.starts_with(prefix.as_str()))
    }
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], model: &WorkspaceModel,
    next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    match hint.value.trim() {
        SOURCE_ADAPTER_BRIEFS => {
            let cfg = ExpectedOperationsConfig::parse(hint.config.as_ref()).ok_or_else(|| {
                HintError::Unsupported {
                    rule_id: rule.rule_id.clone(),
                    kind: HintKind::SetCoverage,
                    reason: "`adapter-briefs` requires a `config: { expected-operations }`",
                }
            })?;
            Ok(match cfg.mode() {
                BriefsMode::Subset => adapter_briefs_subset(rule, candidates, model, &cfg, next_id),
                BriefsMode::Exact => adapter_briefs_exact(rule, candidates, model, &cfg, next_id),
            })
        }
        SOURCE_SKILL_ALLOWED_TOOLS => {
            let cfg = AllowedToolsConfig::parse(rule, hint)?;
            Ok(skill_allowed_tools(rule, candidates, model, &cfg, next_id))
        }
        _ => Err(HintError::Unsupported {
            rule_id: rule.rule_id.clone(),
            kind: HintKind::SetCoverage,
            reason: "unknown set-coverage source discriminator",
        }),
    }
}

/// `mode: subset` — the **expected ⊆ declared** direction: flag every
/// operation the rule expects that the manifest's `briefs.keys()` does
/// not declare. Extra keys are silent.
fn adapter_briefs_subset(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel,
    cfg: &ExpectedOperationsConfig, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);

    let mut out: Vec<Diagnostic> = Vec::new();
    for manifest in &model.adapter_manifests {
        if !candidate_set.contains(&manifest.path) {
            continue;
        }
        let expected = cfg.expected_for(manifest.axis);
        let actual: BTreeSet<&str> = manifest.brief_keys.iter().map(String::as_str).collect();
        let mut missing: Vec<&str> =
            expected.iter().copied().filter(|op| !actual.contains(op)).collect();
        if missing.is_empty() {
            continue;
        }
        missing.sort_unstable();
        let expected_sorted: Vec<String> = expected.iter().map(|s| (*s).to_string()).collect();
        let actual_sorted: Vec<String> = actual.iter().map(|s| (*s).to_string()).collect();
        for op in missing {
            let location = FindingLocation {
                path: manifest.path.clone(),
                line: Some(1),
                column: None,
                end_line: None,
                end_column: None,
            };
            let evidence = FindingEvidence::Structured {
                summary: format!(
                    "adapter '{}' is missing brief for operation '{}'",
                    manifest.name, op,
                ),
                data: serde_json::json!({
                    "adapter": manifest.name,
                    "axis": axis_token(manifest.axis),
                    "missing": op,
                    "expected": expected_sorted,
                    "actual": actual_sorted,
                }),
                locations: None,
            };
            let title = format!(
                "{}: adapter '{}' missing brief for operation '{}'",
                rule.title, manifest.name, op,
            );
            let finding = make_finding(rule, *next_id, title, Some(location), evidence);
            *next_id += 1;
            out.push(finding);
        }
    }
    out
}

/// `mode: exact` — the two-sided **expected == declared** direction:
/// flag both `missing` operations (expected, absent from `briefs.keys()`)
/// and `unexpected` keys (declared, absent from the expected set). One
/// finding per `(adapter, divergence)` pair, emitted in sorted
/// `(divergence, operation)` order for run-to-run determinism.
fn adapter_briefs_exact(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel,
    cfg: &ExpectedOperationsConfig, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);

    let mut out: Vec<Diagnostic> = Vec::new();
    for manifest in &model.adapter_manifests {
        if !candidate_set.contains(&manifest.path) {
            continue;
        }
        let expected = cfg.expected_for(manifest.axis);
        let actual: BTreeSet<&str> = manifest.brief_keys.iter().map(String::as_str).collect();

        let mut divergences: Vec<(&'static str, String)> = Vec::new();
        for op in expected.iter().copied() {
            if !actual.contains(op) {
                divergences.push((DIVERGENCE_MISSING, op.to_owned()));
            }
        }
        for key in actual.iter().copied() {
            if !expected.contains(key) {
                divergences.push((DIVERGENCE_UNEXPECTED, key.to_owned()));
            }
        }
        if divergences.is_empty() {
            continue;
        }
        divergences.sort_unstable();

        let expected_sorted: Vec<String> = expected.iter().map(|s| (*s).to_string()).collect();
        let actual_sorted: Vec<String> = actual.iter().map(|s| (*s).to_string()).collect();
        for (divergence, op) in divergences {
            let location = FindingLocation {
                path: manifest.path.clone(),
                line: Some(1),
                column: None,
                end_line: None,
                end_column: None,
            };
            let evidence = FindingEvidence::Structured {
                summary: format!(
                    "adapter '{}' brief set diverges: {} operation '{}'",
                    manifest.name, divergence, op,
                ),
                data: serde_json::json!({
                    "adapter": manifest.name,
                    "axis": axis_token(manifest.axis),
                    "divergence": divergence,
                    "operation": op,
                    "expected": expected_sorted,
                    "actual": actual_sorted,
                }),
                locations: None,
            };
            let title = format!(
                "{}: adapter '{}' has {} brief operation '{}'",
                rule.title, manifest.name, divergence, op,
            );
            let finding = make_finding(rule, *next_id, title, Some(location), evidence);
            *next_id += 1;
            out.push(finding);
        }
    }
    out
}

/// Flag every `allowed-tools` entry on a candidate skill that the rule's
/// `allowed` set (plus `allowed-prefixes` exemptions) does not cover.
/// One finding per uncovered `(skill, tool)` pair.
fn skill_allowed_tools(
    rule: &ResolvedRule, candidates: &[PathBuf], model: &WorkspaceModel, cfg: &AllowedToolsConfig,
    next_id: &mut u64,
) -> Vec<Diagnostic> {
    let candidate_set = super::candidate_set(candidates);

    let mut out: Vec<Diagnostic> = Vec::new();
    for frontmatter in &model.frontmatter {
        if !candidate_set.contains(&frontmatter.path) {
            continue;
        }
        let Some(tools) = frontmatter.fields.get("allowed-tools").and_then(JsonValue::as_str)
        else {
            continue;
        };
        for tool in tools.split_whitespace().filter(|t| !t.is_empty()) {
            if cfg.covers(tool) {
                continue;
            }
            let location = FindingLocation {
                path: frontmatter.path.clone(),
                line: Some(1),
                column: None,
                end_line: None,
                end_column: None,
            };
            let evidence = FindingEvidence::Structured {
                summary: format!("unrecognised tool '{tool}' in allowed-tools"),
                data: serde_json::json!({
                    "path": frontmatter.path,
                    "tool": tool,
                }),
                locations: None,
            };
            let title = format!("{}: unrecognised tool '{}' in allowed-tools", rule.title, tool);
            let finding = make_finding(rule, *next_id, title, Some(location), evidence);
            *next_id += 1;
            out.push(finding);
        }
    }
    out
}

#[cfg(test)]
mod unit {
    use serde_json::json;

    use super::*;
    use crate::lint::eval::testkit::{candidates, empty_model, hint, hint_with_config, rule};
    use crate::lint::{AdapterAxis, AdapterManifest, Frontmatter};

    fn manifest(name: &str, axis: AdapterAxis, briefs: &[&str]) -> AdapterManifest {
        let axis_dir = match axis {
            AdapterAxis::Sources => "sources",
            AdapterAxis::Targets => "targets",
        };
        AdapterManifest {
            axis,
            name: name.to_string(),
            path: format!("adapters/{axis_dir}/{name}/adapter.yaml"),
            version: None,
            brief_keys: briefs.iter().map(|b| (*b).to_string()).collect(),
        }
    }

    fn skill_frontmatter(path: &str, allowed_tools: &str) -> Frontmatter {
        let mut fields = serde_json::Map::new();
        fields.insert("allowed-tools".to_string(), json!(allowed_tools));
        Frontmatter {
            path: path.to_string(),
            schema_id: None,
            fields,
        }
    }

    // Fixture operation names are deliberately not the real adapter
    // operations: the expected sets are rule-supplied policy, and the
    // `no_embedded_policy` guard rejects real operation-set literals
    // anywhere under `lint/eval/`.
    fn expected_ops() -> serde_json::Value {
        json!({ "expected-operations": { "sources": ["alpha", "beta"], "targets": ["gamma", "delta", "epsilon"] } })
    }

    fn exact_ops() -> serde_json::Value {
        json!({ "mode": "exact", "expected-operations": { "sources": ["alpha", "beta"], "targets": ["gamma", "delta", "epsilon"] } })
    }

    #[test]
    fn missing_operation_flagged_extras_silent() {
        let mut model = empty_model();
        model.adapter_manifests =
            vec![manifest("demo", AdapterAxis::Targets, &["gamma", "delta", "extra"])];
        let cands = candidates(&["adapters/targets/demo/adapter.yaml"]);
        let hint = hint_with_config(HintKind::SetCoverage, "adapter-briefs", Some(expected_ops()));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        // Default `subset` mode: only the missing `epsilon` fires; the
        // `extra` key is `mode: exact`'s job.
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("missing brief for operation 'epsilon'"), "{}", out[0].title);
    }

    #[test]
    fn exact_mode_flags_missing_and_unexpected() {
        let mut model = empty_model();
        model.adapter_manifests =
            vec![manifest("demo", AdapterAxis::Targets, &["gamma", "delta", "rogue"])];
        let cands = candidates(&["adapters/targets/demo/adapter.yaml"]);
        let hint = hint_with_config(HintKind::SetCoverage, "adapter-briefs", Some(exact_ops()));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        let titles: Vec<&str> = out.iter().map(|f| f.title.as_str()).collect();
        assert_eq!(out.len(), 2, "{titles:?}");
        assert!(
            titles.iter().any(|t| t.contains("missing brief operation 'epsilon'")),
            "{titles:?}"
        );
        assert!(
            titles.iter().any(|t| t.contains("unexpected brief operation 'rogue'")),
            "{titles:?}"
        );
    }

    #[test]
    fn exact_mode_silent_on_exact_set() {
        let mut model = empty_model();
        model.adapter_manifests = vec![manifest("demo", AdapterAxis::Sources, &["alpha", "beta"])];
        let cands = candidates(&["adapters/sources/demo/adapter.yaml"]);
        let hint = hint_with_config(HintKind::SetCoverage, "adapter-briefs", Some(exact_ops()));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert!(out.is_empty());
    }

    #[test]
    fn covered_manifest_is_silent() {
        let mut model = empty_model();
        model.adapter_manifests = vec![manifest("demo", AdapterAxis::Sources, &["alpha", "beta"])];
        let cands = candidates(&["adapters/sources/demo/adapter.yaml"]);
        let hint = hint_with_config(HintKind::SetCoverage, "adapter-briefs", Some(expected_ops()));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert!(out.is_empty());
    }

    #[test]
    fn uncovered_tool_flagged() {
        let mut model = empty_model();
        let path = "plugins/p/skills/s/SKILL.md";
        model.frontmatter = vec![skill_frontmatter(path, "Read Write mcp__custom rogue")];
        let cands = candidates(&[path]);
        let cfg = json!({ "allowed": ["Read", "Write"], "allowed-prefixes": ["mcp__"] });
        let hint = hint_with_config(HintKind::SetCoverage, "skill-allowed-tools", Some(cfg));
        let out = evaluate(&rule(), &hint, &cands, &model, &mut 1).expect("evaluate");
        assert_eq!(out.len(), 1);
        assert!(out[0].title.contains("'rogue'"), "{}", out[0].title);
    }

    #[test]
    fn missing_config_is_unsupported() {
        let model = empty_model();
        let hint = hint(HintKind::SetCoverage, "skill-allowed-tools");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }

    #[test]
    fn unknown_source_is_unsupported() {
        let model = empty_model();
        let hint = hint_with_config(HintKind::SetCoverage, "no-such-source", Some(json!({})));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
