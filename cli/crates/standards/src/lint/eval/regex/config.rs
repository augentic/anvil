//! `kind: regex` optional [`RuleHint::config`] payload.

use serde::Deserialize;

use crate::lint::eval::HintError;
use crate::rules::{HintKind, ResolvedRule, RuleHint};

/// Parsed `regex` hint configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegexHintConfig {
    pub negative_match: bool,
    pub capture_group: Option<u32>,
    pub capture_op: Option<CaptureOp>,
    pub capture_value: Option<i64>,
    pub suffix_must_not_start_with: Option<String>,
    pub slash_skill_positional: bool,
    pub join_backslash_continuations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

impl RegexHintConfig {
    pub(crate) fn parse(rule: &ResolvedRule, hint: &RuleHint) -> Result<Self, HintError> {
        let Some(raw) = hint.config.as_ref() else {
            return Ok(Self::default());
        };
        let parsed: RegexHintConfigWire =
            serde_json::from_value(raw.clone()).map_err(|_ignored| HintError::Unsupported {
                rule_id: rule.rule_id.clone(),
                kind: HintKind::Regex,
                reason: "invalid regex hint config JSON",
            })?;
        let any_capture = parsed.capture_group.is_some()
            || parsed.capture_op.is_some()
            || parsed.capture_value.is_some();
        let all_capture = parsed.capture_group.is_some()
            && parsed.capture_op.is_some()
            && parsed.capture_value.is_some();
        if any_capture && !all_capture {
            return Err(HintError::Unsupported {
                rule_id: rule.rule_id.clone(),
                kind: HintKind::Regex,
                reason: "capture-group, capture-op, and capture-value must be set together",
            });
        }
        let capture_op = match parsed.capture_op.as_deref() {
            None => None,
            Some("lt") => Some(CaptureOp::Lt),
            Some("le") => Some(CaptureOp::Le),
            Some("gt") => Some(CaptureOp::Gt),
            Some("ge") => Some(CaptureOp::Ge),
            Some("eq") => Some(CaptureOp::Eq),
            Some(_) => {
                return Err(HintError::Unsupported {
                    rule_id: rule.rule_id.clone(),
                    kind: HintKind::Regex,
                    reason: "unknown capture-op (expected lt, le, gt, ge, eq)",
                });
            }
        };
        Ok(Self {
            negative_match: parsed.negative_match.unwrap_or(false),
            capture_group: parsed.capture_group,
            capture_op,
            capture_value: parsed.capture_value,
            suffix_must_not_start_with: parsed.suffix_must_not_start_with,
            slash_skill_positional: parsed.slash_skill_positional.unwrap_or(false),
            join_backslash_continuations: parsed.join_backslash_continuations.unwrap_or(false),
        })
    }

    pub const fn capture_passes(&self, digits: i64) -> bool {
        let Some(op) = self.capture_op else {
            return true;
        };
        let Some(rhs) = self.capture_value else {
            return true;
        };
        match op {
            CaptureOp::Lt => digits < rhs,
            CaptureOp::Le => digits <= rhs,
            CaptureOp::Gt => digits > rhs,
            CaptureOp::Ge => digits >= rhs,
            CaptureOp::Eq => digits == rhs,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RegexHintConfigWire {
    #[serde(default, rename = "negative-match")]
    negative_match: Option<bool>,
    #[serde(default, rename = "capture-group")]
    capture_group: Option<u32>,
    #[serde(default, rename = "capture-op")]
    capture_op: Option<String>,
    #[serde(default, rename = "capture-value")]
    capture_value: Option<i64>,
    #[serde(default, rename = "suffix-must-not-start-with")]
    suffix_must_not_start_with: Option<String>,
    #[serde(default, rename = "slash-skill-positional")]
    slash_skill_positional: Option<bool>,
    #[serde(default, rename = "join-backslash-continuations")]
    join_backslash_continuations: Option<bool>,
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::rules::{HintKind, RuleHint};

    fn hint_with_config(config: Value) -> RuleHint {
        RuleHint {
            kind: HintKind::Regex,
            value: r"(?i)RFC[-\s]?(\d+)".to_string(),
            description: None,
            config: Some(config),
        }
    }

    fn rule() -> ResolvedRule {
        ResolvedRule {
            rule_id: "CORE-016".to_string(),
            title: "fixture".to_string(),
            severity: specify_diagnostics::Severity::Important,
            trigger: "t".to_string(),
            lint_mode: None,
            applicability: None,
            rule_hints: None,
            references: None,
            origin: crate::rules::Origin::Core,
            path_root: crate::rules::PathRoot::RulesRoot,
            path: "adapters/shared/rules/core/CORE-016.md".to_string(),
            body: String::new(),
            deprecated: None,
        }
    }

    #[test]
    fn parse_capture_threshold() {
        let config = serde_json::json!({
            "capture-group": 1,
            "capture-op": "lt",
            "capture-value": 100
        });
        let parsed = RegexHintConfig::parse(&rule(), &hint_with_config(config)).expect("parses");
        assert_eq!(parsed.capture_group, Some(1));
        assert!(parsed.capture_passes(5));
        assert!(!parsed.capture_passes(3339));
    }
}
