use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::rules::{Deprecated, Origin, PathRoot, Severity};

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
        path: format!("adapters/shared/rules/universal/{id}.md"),
    }
}

fn ids(entries: &[ResolvedRuleEntry]) -> Vec<&str> {
    entries.iter().map(|e| e.rule.id.as_str()).collect()
}

fn ids_of_rules(rules: &[ResolvedRule]) -> Vec<&str> {
    rules.iter().map(|r| r.rule_id.as_str()).collect()
}

/// Test 3: deprecated entries sort after non-deprecated entries
/// regardless of other tie-breakers.
#[test]
fn sort_puts_non_deprecated_first() {
    let mut entries = vec![
        entry("RULE-A", Severity::Important, Origin::Shared, true),
        entry("RULE-A2", Severity::Important, Origin::Shared, false),
    ];
    sort_resolved(&mut entries);
    assert_eq!(ids(&entries), vec!["RULE-A2", "RULE-A"]);
}

/// Test 4: ties on (deprecated, severity, origin) resolve by
/// lexical `rule-id`.
#[test]
fn sort_breaks_ties_by_rule_id() {
    let mut entries = vec![
        entry("OMNIA-002", Severity::Critical, Origin::Target, false),
        entry("OMNIA-001", Severity::Critical, Origin::Target, false),
        entry("OMNIA-003", Severity::Critical, Origin::Target, false),
    ];
    sort_resolved(&mut entries);
    assert_eq!(ids(&entries), vec!["OMNIA-001", "OMNIA-002", "OMNIA-003"]);
}

/// Test 5: full-tuple precedence — deprecation dominates severity
/// dominates origin dominates id. Walks through a mix that
/// triggers every comparator dimension.
#[test]
fn sort_full_tuple_precedence() {
    let mut entries = vec![
        entry("A", Severity::Critical, Origin::Target, true),
        entry("Z", Severity::Optional, Origin::Shared, false),
        entry("M", Severity::Critical, Origin::Source, false),
    ];
    sort_resolved(&mut entries);
    // Z (non-deprecated, Optional, Shared) and M (non-deprecated,
    // Critical, Source) both beat A (deprecated). M's Critical
    // beats Z's Optional.
    assert_eq!(ids(&entries), vec!["M", "Z", "A"]);
}

/// Helper: a minimal frontmatter + body that parses through CH-11
/// and validates against the codex-rule schema.
fn rule_markdown(id: &str, title: &str, severity: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: {title}\nseverity: {severity}\ntrigger: Synthetic CH-14 build_resolved_rules fixture trigger sentence long enough for schema.\n---\n\n## Rule\n\nBody for {id}.\n"
    )
}

fn write_rule(path: &Path, id: &str, title: &str, severity: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, rule_markdown(id, title, severity)).expect("write rule fixture");
}

fn run_build(rules_root: &Path, project_dir: &Path) -> ResolvedRules {
    let sources: Vec<String> = Vec::new();
    let inputs = ResolveInputs {
        project_dir,
        rules_root: Some(rules_root),
        target_adapter: "omnia",
        source_adapters: &sources,
        artifact_paths: &[],
        languages: &[],
        include_deprecated: false,
        include_unmatched: false,
        include_core: false,
    };
    build_resolved_rules(&inputs).expect("build_resolved_rules succeeds")
}

/// Test 6: `build_resolved_rules` integration — wire envelope is
/// versioned, target/source carry through, and rules emerge
/// sorted per the closed four-tuple.
#[test]
fn build_emits_versioned_envelope() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-002.md"),
        "UNI-002",
        "Important shared",
        "important",
    );
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Critical shared",
        "critical",
    );
    write_rule(
        &project.path().join("adapters/targets/omnia/rules/omnia-001.md"),
        "OMNIA-001",
        "Important target",
        "important",
    );

    let resolved = run_build(rules_root.path(), project.path());

    assert_eq!(resolved.version, 1);
    assert_eq!(resolved.target_adapter, "omnia");
    assert!(resolved.source_adapters.is_empty());
    assert_eq!(resolved.rules.len(), 3);
    // UNI-001 is Critical (beats OMNIA-001 Important); OMNIA-001
    // is Important + Target (beats UNI-002 Important + Shared);
    // UNI-002 trails.
    assert_eq!(ids_of_rules(&resolved.rules), vec!["UNI-001", "OMNIA-001", "UNI-002"]);
}

/// Test 7: paths on the wire envelope are anchored to `path-root`
/// and never absolute (no leading `/` on Unix, no `<drive>:` on
/// Windows). Guards the cross-platform determinism the plan calls
/// out.
#[test]
fn paths_anchored_not_absolute() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared",
        "important",
    );
    write_rule(
        &project.path().join("adapters/targets/omnia/rules/omnia-001.md"),
        "OMNIA-001",
        "Target",
        "important",
    );

    let resolved = run_build(rules_root.path(), project.path());
    for rule in &resolved.rules {
        assert!(
            !rule.path.starts_with('/'),
            "rule {} path leaked an absolute prefix: {}",
            rule.rule_id,
            rule.path,
        );
        // Windows drive-letter guard: a `<letter>:` prefix would
        // mean the resolver failed to strip the temp dir root.
        let bytes = rule.path.as_bytes();
        let drive_letter = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        assert!(
            !drive_letter,
            "rule {} path leaked a Windows drive prefix: {}",
            rule.rule_id, rule.path,
        );
        assert!(
            !rule.path.contains('\\'),
            "rule {} path leaked a backslash separator: {}",
            rule.rule_id,
            rule.path,
        );
    }
}

/// Test 8: identical inputs produce byte-identical JSON across
/// runs. Pins the stability guarantee CH-17 will rely on for
/// golden tests.
#[test]
fn build_byte_stable() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared",
        "critical",
    );
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-002.md"),
        "UNI-002",
        "Shared opt",
        "optional",
    );
    write_rule(
        &project.path().join("adapters/targets/omnia/rules/omnia-001.md"),
        "OMNIA-001",
        "Target",
        "important",
    );

    let first = run_build(rules_root.path(), project.path());
    let second = run_build(rules_root.path(), project.path());
    let first_json = serde_json::to_string(&first).expect("serialise first");
    let second_json = serde_json::to_string(&second).expect("serialise second");
    assert_eq!(first_json, second_json);
}
