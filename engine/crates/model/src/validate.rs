//! Validation rule registry and runner.
//!
//! `Rule` / `CrossRule` declare their `Classification`; [`validate_slice`]
//! returns a `Vec<Diagnostic>` — the neutral currency shared with the
//! `lint` surface. Structural `Fail` outcomes become deterministic
//! `violation` diagnostics (`important`, blocking); semantic rules become
//! non-blocking `review` diagnostics (`suggestion`,
//! [`specify_diagnostics::DiagnosticKind::Review`]) that ask the agent to
//! apply judgment. Passing structural rules emit no diagnostic — the
//! report carries only findings, never the full pass checklist.

use std::path::Path;

use crate::spec::ParsedSpec;
use crate::task::Progress;

mod primitives;
mod registry;
mod run;

pub use run::validate_slice;

/// How the CLI decides a rule's outcome — declared at the rule's
/// definition site.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Classification {
    /// CLI decides Pass/Fail deterministically; `Fail` becomes a
    /// deterministic `violation` diagnostic.
    Structural,
    /// CLI cannot decide; emits a non-blocking `review` diagnostic that
    /// asks the agent to apply judgment.
    Semantic,
}

/// Outcome of invoking a structural rule's `check` function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleOutcome {
    /// The rule passed.
    Pass,
    /// The rule failed with an explanation.
    Fail {
        /// Human-readable failure detail.
        detail: String,
    },
}

/// A named rule attached to a specific brief id.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    /// Stable dot-namespaced identifier (e.g. `proposal.why-has-content`).
    pub id: &'static str,
    /// Human-readable description of what the rule checks.
    pub description: &'static str,
    /// Whether the rule is structural or semantic.
    pub classification: Classification,
    /// `Some` for `Classification::Structural`; `None` for `Semantic`,
    /// which the runner always materialises as `Deferred`.
    pub check: Option<fn(&BriefContext<'_>) -> RuleOutcome>,
}

/// Inputs a brief-scoped structural checker needs.
#[derive(Debug)]
pub struct BriefContext<'a> {
    /// The brief id being validated.
    pub id: &'a str,
    /// Artifact file content.
    pub content: &'a str,
    /// Parsed spec (when `brief_id == "specs"`).
    pub parsed_spec: Option<&'a ParsedSpec>,
    /// Parsed task progress (when `brief_id == "tasks"`).
    pub tasks: Option<&'a Progress>,
    /// Absolute path to the slice directory.
    pub slice_dir: &'a Path,
    /// Absolute path to the specs directory.
    pub specs_dir: &'a Path,
}

/// A rule that spans multiple briefs.
#[derive(Debug, Clone, Copy)]
pub struct CrossRule {
    /// Stable dot-namespaced identifier (e.g. `cross.proposal-domains-have-specs`).
    pub id: &'static str,
    /// Human-readable description of what the rule checks.
    pub description: &'static str,
    /// Whether the rule is structural or semantic.
    pub classification: Classification,
    /// Checker function — only invoked for structural rules.
    pub check: fn(&CrossContext<'_>) -> RuleOutcome,
}

/// Inputs a cross-brief checker needs.
#[derive(Debug)]
pub struct CrossContext<'a> {
    /// Absolute path to the slice directory.
    pub slice_dir: &'a Path,
    /// Absolute path to the specs directory.
    pub specs_dir: &'a Path,
}

#[cfg(test)]
mod tests {
    use super::registry::{cross_rules, rules_for};

    /// `rules_for` returns empty for unknown brief ids.
    #[test]
    fn unknown_brief_no_rules() {
        assert!(rules_for("unknown-brief-id").is_empty());
        assert!(rules_for("").is_empty());
    }

    #[test]
    fn min_rules_per_brief() {
        assert!(rules_for("proposal").len() >= 3);
        assert!(rules_for("specs").len() >= 4);
        assert!(!rules_for("design").is_empty());
        assert!(rules_for("tasks").len() >= 2);
        assert!(rules_for("contracts").len() >= 3);
        assert!(cross_rules().len() >= 3);
    }

    /// Every rule carries a stable `<brief>.<kebab>` id.
    #[test]
    fn rule_ids_are_namespaced() {
        for (brief, prefix) in &[
            ("proposal", "proposal."),
            ("specs", "specs."),
            ("design", "design."),
            ("tasks", "tasks."),
            ("contracts", "contracts."),
        ] {
            for rule in rules_for(brief) {
                assert!(
                    rule.id.starts_with(prefix),
                    "rule id `{}` should start with `{}`",
                    rule.id,
                    prefix
                );
            }
        }
        for rule in cross_rules() {
            assert!(rule.id.starts_with("cross."));
        }
    }
}
