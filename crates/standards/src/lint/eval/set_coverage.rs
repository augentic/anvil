//! `kind: set-coverage` evaluator.
//!
//! Asserts that the set of values some candidate file declares
//! covers a closed expected set.
//!
//! Source discriminators that ship today (selected by the hint's
//! `value`):
//!
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

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value as JsonValue;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

const SOURCE_SKILL_ALLOWED_TOOLS: &str = "skill-allowed-tools";

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
    use crate::lint::Frontmatter;
    use crate::lint::eval::testkit::{candidates, empty_model, hint, hint_with_config, rule};

    fn skill_frontmatter(path: &str, allowed_tools: &str) -> Frontmatter {
        let mut fields = serde_json::Map::new();
        fields.insert("allowed-tools".to_string(), json!(allowed_tools));
        Frontmatter {
            path: path.to_string(),
            schema_id: None,
            fields,
        }
    }

    #[test]
    fn flags_set_divergences() {
        // `skill-allowed-tools`: a tool neither in `allowed` nor matching an
        // `allowed-prefixes` exemption is flagged; covered/prefixed are silent.
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
    fn rejects_bad_config() {
        let model = empty_model();
        // A selector with no `config` is rejected.
        let hint = hint(HintKind::SetCoverage, "skill-allowed-tools");
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
        // An unknown source discriminator is rejected.
        let hint = hint_with_config(HintKind::SetCoverage, "no-such-source", Some(json!({})));
        evaluate(&rule(), &hint, &[], &model, &mut 1).unwrap_err();
    }
}
