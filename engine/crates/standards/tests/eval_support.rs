//! Shared scaffolding for the hint-umbrella integration tests.
//!
//! The `lint_hint` integration crate `mod`s this file to share the
//! [`NoToolRunner`] / `make_rule` / `hint` plumbing. The scaffolding
//! lives under `tests/<helper>/mod.rs` per Rust's idiom; the host
//! workspace forbids `mod.rs` everywhere else (see
//! `docs/standards/coding-standards.md` §"Module layout").

use std::path::Path;

use specify_diagnostics::Severity;
use specify_standards::lint::eval::{ToolOutput, ToolRunError, ToolRunner};
use specify_standards::rules::{HintKind, Origin, PathRoot, ResolvedRule, RuleHint};

pub fn make_rule(rule_id: &str, hints: Vec<RuleHint>) -> ResolvedRule {
    ResolvedRule {
        rule_id: rule_id.to_string(),
        title: format!("{rule_id} test rule"),
        severity: Severity::Important,
        trigger: format!("Trigger for {rule_id}"),
        lint_mode: None,
        applicability: None,
        rule_hints: if hints.is_empty() { None } else { Some(hints) },
        references: None,
        origin: Origin::Shared,
        path_root: PathRoot::RulesRoot,
        path: format!("shared/{rule_id}.md"),
        body: String::new(),
        deprecated: None,
    }
}

pub fn hint(kind: HintKind, value: &str) -> RuleHint {
    RuleHint {
        kind,
        value: value.to_string(),
        description: None,
        config: None,
    }
}

pub struct NoToolRunner;

impl ToolRunner for NoToolRunner {
    fn run(
        &self, _tool_name: &str, _args: &[String], _project_dir: &Path,
    ) -> Result<ToolOutput, ToolRunError> {
        Err(ToolRunError::Runtime("no tool runner wired".to_string()))
    }

    fn is_declared(&self, _tool_name: &str) -> bool {
        false
    }
}
