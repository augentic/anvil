use super::*;
use crate::rules::{Deprecated, Origin, PathRoot, Severity};

// The `build_resolved_rules` envelope test is re-homed to the crate's public
// API in `crates/standards/tests/resolve_sort.rs`; the path-anchoring and
// byte-stability properties are asserted end-to-end by
// `tests/rules.rs::export::{paths_anchored_not_absolute,
// stable_ordering_byte_identical}`. What stays here is the in-memory
// `sort_resolved` comparator matrix, collapsed from three tests into one —
// every former input is preserved.

fn rule(id: &str, severity: Severity, deprecated: bool) -> Rule {
    Rule {
        id: id.into(),
        title: format!("{id} fixture"),
        severity,
        trigger: "Synthetic CH-14 sort fixture trigger sentence long enough for schema.".into(),
        lint_mode: None,
        applicability: None,
        rule_hints: None,
        references: None,
        deprecated: deprecated.then(|| Deprecated {
            reason: "fixture deprecation".into(),
            replaced_by: None,
        }),
        body: format!("## Rule\n\nBody for {id}.\n"),
    }
}

fn entry(id: &str, severity: Severity, origin: Origin, deprecated: bool) -> ResolvedRuleEntry {
    ResolvedRuleEntry {
        rule: rule(id, severity, deprecated),
        origin,
        path_root: PathRoot::RulesRoot,
        path: format!("adapters/codex/rules/universal/{id}.md"),
    }
}

fn ids(entries: &[ResolvedRuleEntry]) -> Vec<&str> {
    entries.iter().map(|e| e.rule.id.as_str()).collect()
}

/// The `sort_resolved` comparator: deprecation dominates severity dominates
/// origin dominates lexical `rule-id`. Collapsed from the three former
/// single-dimension tests.
#[test]
fn sort_resolved_full_precedence() {
    // Deprecated entries sort after non-deprecated, all else equal.
    let mut entries = vec![
        entry("RULE-A", Severity::Important, Origin::Shared, true),
        entry("RULE-A2", Severity::Important, Origin::Shared, false),
    ];
    sort_resolved(&mut entries);
    assert_eq!(ids(&entries), vec!["RULE-A2", "RULE-A"]);

    // Ties on (deprecated, severity, origin) resolve by lexical `rule-id`.
    let mut entries = vec![
        entry("ORG-002", Severity::Critical, Origin::Target, false),
        entry("ORG-001", Severity::Critical, Origin::Target, false),
        entry("ORG-003", Severity::Critical, Origin::Target, false),
    ];
    sort_resolved(&mut entries);
    assert_eq!(ids(&entries), vec!["ORG-001", "ORG-002", "ORG-003"]);

    // Full-tuple precedence across a mix that triggers every comparator
    // dimension: Z (non-deprecated, Optional, Shared) and M (non-deprecated,
    // Critical, Source) both beat A (deprecated); M's Critical beats Z's
    // Optional.
    let mut entries = vec![
        entry("A", Severity::Critical, Origin::Target, true),
        entry("Z", Severity::Optional, Origin::Shared, false),
        entry("M", Severity::Critical, Origin::Source, false),
    ];
    sort_resolved(&mut entries);
    assert_eq!(ids(&entries), vec!["M", "Z", "A"]);
}
